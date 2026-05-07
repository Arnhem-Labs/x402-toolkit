//! High-level [`X402Client`] — `reqwest`-style HTTP requests that
//! transparently handle `402 Payment Required`.
//!
//! The client probes a target URL, decodes the [`PaymentRequired`]
//! challenge from `X-PAYMENT-REQUIRED`, signs the first acceptable spec
//! with a [`WalletSigner`], retries with the `X-PAYMENT` header, and
//! returns the final response.

use std::sync::Arc;

use x402_toolkit_types::{headers, PaymentRequired};

use crate::{sign_authorization, ClientError, WalletSigner};

/// High-level x402 client.
///
/// # Example
///
/// ```no_run
/// # tokio_test::block_on(async {
/// use x402_toolkit_client::{LocalSigner, X402Client};
///
/// let signer = LocalSigner::from_hex(&std::env::var("TEST_PRIVATE_KEY").unwrap()).unwrap();
/// let client = X402Client::new(signer);
/// let resp = client.get("https://api.example.com/v1/chat").send().await.unwrap();
/// println!("{}", resp.status());
/// # });
/// ```
pub struct X402Client<S: WalletSigner> {
    http: reqwest::Client,
    signer: Arc<S>,
}

// Manual `Clone` impl: `Arc<S>` is always cheap to clone; we don't need a
// `S: Clone` bound.
impl<S: WalletSigner> Clone for X402Client<S> {
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            signer: Arc::clone(&self.signer),
        }
    }
}

impl<S: WalletSigner> X402Client<S> {
    /// Build a new client with a default `reqwest::Client`.
    pub fn new(signer: S) -> Self {
        Self {
            http: reqwest::Client::new(),
            signer: Arc::new(signer),
        }
    }

    /// Customize the underlying HTTP client.
    pub fn with_http_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    /// Begin a `GET` request that will transparently handle 402.
    pub fn get(&self, url: impl reqwest::IntoUrl) -> RequestBuilder<S> {
        RequestBuilder {
            client: self.clone(),
            method: reqwest::Method::GET,
            url: url.into_url().expect("valid URL"),
            json_body: None,
            headers: Vec::new(),
        }
    }

    /// Begin a `POST` request that will transparently handle 402.
    pub fn post(&self, url: impl reqwest::IntoUrl) -> RequestBuilder<S> {
        RequestBuilder {
            client: self.clone(),
            method: reqwest::Method::POST,
            url: url.into_url().expect("valid URL"),
            json_body: None,
            headers: Vec::new(),
        }
    }
}

/// Builder for an x402-aware request.
pub struct RequestBuilder<S: WalletSigner> {
    client: X402Client<S>,
    method: reqwest::Method,
    url: reqwest::Url,
    json_body: Option<serde_json::Value>,
    headers: Vec<(String, String)>,
}

impl<S: WalletSigner> RequestBuilder<S> {
    /// Set a JSON body.
    pub fn json<T: serde::Serialize>(mut self, body: &T) -> Result<Self, ClientError> {
        self.json_body = Some(serde_json::to_value(body)?);
        Ok(self)
    }

    /// Add a header.
    pub fn header(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.headers.push((k.into(), v.into()));
        self
    }

    /// Send the request, signing and retrying on 402.
    pub async fn send(self) -> Result<reqwest::Response, ClientError> {
        let resp = self.send_once(None).await?;
        if resp.status() != reqwest::StatusCode::PAYMENT_REQUIRED {
            return Ok(resp);
        }
        let header = resp
            .headers()
            .get(headers::X_PAYMENT_REQUIRED)
            .ok_or_else(|| ClientError::Protocol("402 without X-PAYMENT-REQUIRED header".into()))?
            .to_str()
            .map_err(|e| ClientError::Protocol(format!("X-PAYMENT-REQUIRED not utf-8: {e}")))?
            .to_string();
        let challenge: PaymentRequired = headers::decode_payment_required(&header)?;
        let spec =
            challenge.accepts.into_iter().next().ok_or_else(|| {
                ClientError::Protocol("PaymentRequired had empty 'accepts'".into())
            })?;
        let payload = sign_authorization(self.client.signer.as_ref(), &spec).await?;
        let payment_header = headers::encode_payment(&payload)?;
        self.send_once(Some(payment_header)).await
    }

    async fn send_once(
        &self,
        payment_header: Option<String>,
    ) -> Result<reqwest::Response, ClientError> {
        let mut req = self
            .client
            .http
            .request(self.method.clone(), self.url.clone());
        for (k, v) in &self.headers {
            req = req.header(k, v);
        }
        if let Some(p) = payment_header {
            req = req.header(headers::X_PAYMENT, p);
        }
        if let Some(body) = &self.json_body {
            req = req.json(body);
        }
        let resp = req.send().await?;
        Ok(resp)
    }
}
