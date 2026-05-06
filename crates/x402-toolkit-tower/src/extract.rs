//! Header parsing and 402-response building, factored out of the
//! middleware so they're useful in handler code too.

use http::{header::HeaderValue, Response, StatusCode};
use http_body_util::Full;

use x402_toolkit_types::{
    headers, PaymentPayload, PaymentRequired, PaymentSpec,
};

/// Build a `PaymentRequired` challenge for `spec` and serialize it as a
/// full HTTP `402` response body, including the `X-PAYMENT-REQUIRED`
/// header.
///
/// The returned body type is `http_body_util::Full<bytes::Bytes>` so the
/// response composes with most tower / hyper / axum stacks out of the
/// box. Callers who need a different body type can rebuild the response
/// from the [`PaymentRequired`] value via [`PaymentRequired::single`] /
/// [`headers::encode_payment_required`].
pub fn build_402_response(
    spec: &PaymentSpec,
) -> Response<Full<bytes::Bytes>> {
    let pr = PaymentRequired::single(spec.clone());
    let body_bytes = serde_json::to_vec(&pr).expect("PaymentRequired serializes");
    let header = headers::encode_payment_required(&pr).expect("PaymentRequired base64-encodes");

    let mut resp = Response::new(Full::new(body_bytes.into()));
    *resp.status_mut() = StatusCode::PAYMENT_REQUIRED;
    resp.headers_mut().insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(v) = HeaderValue::from_str(&header) {
        resp.headers_mut()
            .insert(headers::X_PAYMENT_REQUIRED, v);
    }
    resp
}

/// Parse the `X-PAYMENT` header, if present.
///
/// Returns `Ok(None)` if the header is absent, `Ok(Some(_))` if it's
/// present and well-formed, and `Err(_)` if it's present but malformed.
pub fn parse_payment_header(
    headers: &http::HeaderMap,
) -> Result<Option<PaymentPayload>, x402_toolkit_types::X402Error> {
    let Some(v) = headers.get(headers::X_PAYMENT) else {
        return Ok(None);
    };
    let s = v
        .to_str()
        .map_err(|e| x402_toolkit_types::X402Error::Invalid(format!("X-PAYMENT not utf-8: {e}")))?;
    Ok(Some(headers::decode_payment(s)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use x402_toolkit_types::{Network, PaymentSpec};

    #[test]
    fn build_402_sets_status_and_header() {
        let spec = PaymentSpec::usdc(Network::BaseSepolia, "1000", "0xVAULT");
        let resp = build_402_response(&spec);
        assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
        assert!(resp.headers().contains_key(headers::X_PAYMENT_REQUIRED));
        assert_eq!(
            resp.headers()
                .get(http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
    }

    #[test]
    fn parse_absent_header_returns_none() {
        let h = http::HeaderMap::new();
        assert!(parse_payment_header(&h).unwrap().is_none());
    }
}
