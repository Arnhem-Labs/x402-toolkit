//! Axum extractor that pulls the verified [`PaymentReceipt`] out of
//! request extensions.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

use x402_toolkit_types::PaymentReceipt;

/// Newtype around [`PaymentReceipt`] usable as an axum extractor.
///
/// `X402Layer` attaches the receipt to request extensions after a
/// payment is verified. Any handler downstream of the layer can pull it
/// out via this extractor.
///
/// # Example
///
/// ```
/// use x402_toolkit_axum::Receipt;
///
/// async fn handler(Receipt(r): Receipt) -> String {
///     format!("paid by {} on {}", r.payer, r.network.caip2())
/// }
/// ```
///
/// If the extractor is used outside an `X402Layer`-gated route, it
/// returns `500 Internal Server Error` (the receipt extension is
/// missing). Use [`OptionalReceipt`] if you want a `None` instead.
#[derive(Debug, Clone)]
pub struct Receipt(pub PaymentReceipt);

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Receipt {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<PaymentReceipt>()
            .cloned()
            .map(Receipt)
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "x402-toolkit-axum::Receipt extractor requires X402Layer to be applied",
            ))
    }
}

/// Like [`Receipt`] but yields `None` when the receipt extension is
/// absent (instead of erroring). Useful for routes that are optionally
/// gated (e.g. free tier + paid tier behind the same handler).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OptionalReceipt(pub Option<PaymentReceipt>);

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for OptionalReceipt {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalReceipt(parts.extensions.get::<PaymentReceipt>().cloned()))
    }
}
