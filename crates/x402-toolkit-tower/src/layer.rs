//! [`X402Layer`] — the middleware that gates an inner `tower::Service` on
//! valid x402 payments.
//!
//! See the crate root for high-level docs.

use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body::Body as HttpBody;
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full};
use pin_project_lite::pin_project;
use tower_layer::Layer;
use tower::Service;
use tracing::{debug, warn};

use x402_toolkit_client::{ClientError, Facilitator};
use x402_toolkit_types::{headers, PaymentSpec};

use crate::{
    extract::{build_402_response, parse_payment_header},
    store::{InMemoryStore, ReceiptStore, StoreError},
};

/// Boxed, unified response body. Inner-service bodies and the
/// middleware's own 402 body are both wrapped into this type so the
/// returned response body has a single concrete type.
pub type X402Body = UnsyncBoxBody<Bytes, BoxError>;

/// Boxed error type used by [`X402Body`].
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Middleware configuration.
///
/// Hold a clone of this on the layer; cloning is cheap (`Arc`-backed
/// internally).
#[derive(Clone)]
pub struct LayerConfig {
    spec: PaymentSpec,
    facilitator: Arc<dyn Facilitator>,
    store: Arc<dyn ReceiptStore>,
}

impl LayerConfig {
    /// Build a config that gates routes on `spec`, verifying with
    /// `facilitator` and using an [`InMemoryStore`] for nonce-replay
    /// protection.
    pub fn new<F: Facilitator>(spec: PaymentSpec, facilitator: F) -> Self {
        Self {
            spec,
            facilitator: Arc::new(facilitator),
            store: Arc::new(InMemoryStore::new()),
        }
    }

    /// Override the [`ReceiptStore`].
    pub fn with_store<S: ReceiptStore>(mut self, store: S) -> Self {
        self.store = Arc::new(store);
        self
    }

    /// Borrow the configured spec.
    pub fn spec(&self) -> &PaymentSpec {
        &self.spec
    }
}

/// Tower layer for x402 payment gating.
///
/// # Example
///
/// ```
/// use x402_toolkit_client::MockFacilitator;
/// use x402_toolkit_tower::{LayerConfig, X402Layer};
/// use x402_toolkit_types::{Network, PaymentSpec};
///
/// let cfg = LayerConfig::new(
///     PaymentSpec::usdc(Network::BaseSepolia, "1000", "0x9876543210987654321098765432109876543210"),
///     MockFacilitator::default(),
/// );
/// let layer = X402Layer::new(cfg);
/// ```
#[derive(Clone)]
pub struct X402Layer {
    config: LayerConfig,
}

impl X402Layer {
    /// Build a layer from a [`LayerConfig`].
    pub fn new(config: LayerConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for X402Layer {
    type Service = X402Service<S>;

    fn layer(&self, inner: S) -> Self::Service {
        X402Service {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Service produced by [`X402Layer`].
#[derive(Clone)]
pub struct X402Service<S> {
    inner: S,
    config: LayerConfig,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for X402Service<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: HttpBody<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
{
    type Response = Response<X402Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Clone the inner service so we can move it into the future.
        // Avoids the not-`Sync`-via-&mut-self cloneable-tower-service trap.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let config = self.config.clone();

        Box::pin(async move {
            // 1. Try to decode `X-PAYMENT`.
            let payload = match parse_payment_header(req.headers()) {
                Ok(Some(p)) => p,
                Ok(None) => {
                    debug!("no X-PAYMENT header — returning 402");
                    return Ok(into_x402_body_response(build_402_response(&config.spec)));
                }
                Err(e) => {
                    warn!(error = %e, "malformed X-PAYMENT header");
                    return Ok(into_x402_body_response(build_402_response(&config.spec)));
                }
            };

            // 2. Verify with the facilitator.
            let receipt = match config.facilitator.verify(&payload).await {
                Ok(r) => r,
                Err(ClientError::Rejected(reason)) => {
                    warn!(%reason, "facilitator rejected payment");
                    return Ok(into_x402_body_response(build_402_response(&config.spec)));
                }
                Err(other) => {
                    warn!(error = %other, "facilitator error — returning 502");
                    return Ok(plain_response(
                        StatusCode::BAD_GATEWAY,
                        "x402 facilitator error",
                    ));
                }
            };

            // 3. Replay-check the nonce.
            if let Ok(nonce) = payload.payload.authorization.nonce_bytes() {
                match config.store.mark_seen(&receipt, &nonce).await {
                    Ok(()) => {}
                    Err(StoreError::Replayed) => {
                        warn!("nonce replayed");
                        return Ok(into_x402_body_response(build_402_response(&config.spec)));
                    }
                    Err(StoreError::Backend(e)) => {
                        warn!(error = %e, "receipt store backend error; allowing through");
                        // Fail-open: if the store is down, don't block
                        // legitimate paid traffic. Production deployments
                        // should monitor this log.
                    }
                }
            }

            // 4. Attach receipt to request extensions and forward.
            let mut req = req;
            req.extensions_mut().insert(receipt.clone());

            // S::Error is Infallible — the call cannot fail.
            let response = match inner.call(req).await {
                Ok(r) => r,
                Err(never) => match never {},
            };
            let mut response = response.map(|b| b.map_err(Into::into).boxed_unsync());

            // 5. Add `X-PAYMENT-RESPONSE` to the inner response.
            if let Ok(encoded) = headers::encode_receipt(&receipt) {
                if let Ok(v) = http::HeaderValue::from_str(&encoded) {
                    response
                        .headers_mut()
                        .insert(headers::X_PAYMENT_RESPONSE, v);
                }
            }

            Ok(response)
        })
    }
}

/// Build a tiny plain-text response with the given status code.
fn plain_response(status: StatusCode, body: &'static str) -> Response<X402Body> {
    let mut resp = Response::new(
        Full::new(Bytes::from_static(body.as_bytes()))
            .map_err(|never| match never {})
            .boxed_unsync(),
    );
    *resp.status_mut() = status;
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    resp
}

/// Wrap the [`build_402_response`] body (`Full<Bytes>`) into [`X402Body`].
fn into_x402_body_response(resp: Response<Full<Bytes>>) -> Response<X402Body> {
    resp.map(|b| {
        b.map_err(|never| match never {})
            .boxed_unsync()
    })
}

pin_project! {
    /// Reserved future shape for breaking-change-free upgrades.
    /// Currently unused; the service's `Future` is `Pin<Box<...>>`.
    #[doc(hidden)]
    pub struct ResponseFuture<F> {
        #[pin]
        inner: F,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use x402_toolkit_client::{sign_authorization, LocalSigner, MockFacilitator};
    use x402_toolkit_types::{Network, PaymentSpec};

    fn config() -> LayerConfig {
        LayerConfig::new(
            PaymentSpec::usdc(
                Network::BaseSepolia,
                "1000",
                "0x9876543210987654321098765432109876543210",
            )
            .with_resource("https://example.com/api"),
            MockFacilitator::default(),
        )
    }

    /// Hand-rolled "always-200 OK" service to keep type inference simple.
    #[derive(Clone)]
    struct Echo;

    impl Service<Request<Full<Bytes>>> for Echo {
        type Response = Response<Full<Bytes>>;
        type Error = Infallible;
        type Future =
            Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: Request<Full<Bytes>>) -> Self::Future {
            Box::pin(async {
                Ok(Response::new(Full::new(Bytes::from_static(b"OK"))))
            })
        }
    }

    async fn body_to_string(b: X402Body) -> String {
        let collected = b.collect().await.unwrap();
        let bytes = collected.to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn drive_once(
        svc: &mut X402Service<Echo>,
        req: Request<Full<Bytes>>,
    ) -> Response<X402Body> {
        std::future::poll_fn(|cx| Service::<Request<Full<Bytes>>>::poll_ready(svc, cx))
            .await
            .unwrap();
        Service::<Request<Full<Bytes>>>::call(svc, req).await.unwrap()
    }

    #[tokio::test]
    async fn missing_payment_returns_402() {
        let mut svc = X402Layer::new(config()).layer(Echo);
        let req = Request::builder()
            .uri("/api")
            .body(Full::new(Bytes::new()))
            .unwrap();
        let resp = drive_once(&mut svc, req).await;
        assert_eq!(resp.status(), http::StatusCode::PAYMENT_REQUIRED);
        assert!(resp.headers().contains_key(headers::X_PAYMENT_REQUIRED));
    }

    #[tokio::test]
    async fn valid_payment_passes_through_with_response_header() {
        let mut svc = X402Layer::new(config()).layer(Echo);

        let signer = LocalSigner::random();
        let payload = sign_authorization(&signer, config().spec()).await.unwrap();
        let header_val = headers::encode_payment(&payload).unwrap();

        let req = Request::builder()
            .uri("/api")
            .header(headers::X_PAYMENT, header_val)
            .body(Full::new(Bytes::new()))
            .unwrap();
        let resp = drive_once(&mut svc, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert!(resp.headers().contains_key(headers::X_PAYMENT_RESPONSE));
        assert_eq!(body_to_string(resp.into_body()).await, "OK");
    }

    #[tokio::test]
    async fn nonce_replay_is_rejected() {
        let cfg = config();
        let mut svc = X402Layer::new(cfg.clone()).layer(Echo);

        let signer = LocalSigner::random();
        let payload = sign_authorization(&signer, cfg.spec()).await.unwrap();
        let header_val = headers::encode_payment(&payload).unwrap();

        let req1 = Request::builder()
            .uri("/api")
            .header(headers::X_PAYMENT, header_val.clone())
            .body(Full::new(Bytes::new()))
            .unwrap();
        let resp1 = drive_once(&mut svc, req1).await;
        assert_eq!(resp1.status(), http::StatusCode::OK);

        let req2 = Request::builder()
            .uri("/api")
            .header(headers::X_PAYMENT, header_val)
            .body(Full::new(Bytes::new()))
            .unwrap();
        let resp2 = drive_once(&mut svc, req2).await;
        assert_eq!(resp2.status(), http::StatusCode::PAYMENT_REQUIRED);
    }
}
