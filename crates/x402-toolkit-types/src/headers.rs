//! Encoding and decoding helpers for the `X-PAYMENT-REQUIRED`,
//! `X-PAYMENT`, and `X-PAYMENT-RESPONSE` headers.
//!
//! The on-the-wire form is `base64(serde_json::to_vec(value))`. These
//! helpers are tiny enough to inline at call sites, but isolating them
//! here lets us swap in canonical-JSON later without touching consumers.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{de::DeserializeOwned, Serialize};

use crate::{PaymentPayload, PaymentReceipt, PaymentRequired, X402Error};

/// HTTP header name carrying the server's `PaymentRequired` challenge.
pub const X_PAYMENT_REQUIRED: &str = "X-PAYMENT-REQUIRED";
/// HTTP header name carrying the client's signed [`PaymentPayload`].
pub const X_PAYMENT: &str = "X-PAYMENT";
/// HTTP header name carrying the server's [`PaymentReceipt`] on success.
pub const X_PAYMENT_RESPONSE: &str = "X-PAYMENT-RESPONSE";

/// Encode any `Serialize` value as `base64(json_bytes)` for use as an HTTP
/// header value.
pub fn encode_header<T: Serialize>(value: &T) -> Result<String, X402Error> {
    let json = serde_json::to_vec(value)?;
    Ok(STANDARD.encode(json))
}

/// Decode an `X-*` header value back into the wire type.
pub fn decode_header<T: DeserializeOwned>(value: &str) -> Result<T, X402Error> {
    let bytes = STANDARD.decode(value.trim())?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Convenience — encode a [`PaymentRequired`] for `X-PAYMENT-REQUIRED`.
pub fn encode_payment_required(p: &PaymentRequired) -> Result<String, X402Error> {
    encode_header(p)
}

/// Convenience — decode an `X-PAYMENT-REQUIRED` header into a [`PaymentRequired`].
pub fn decode_payment_required(value: &str) -> Result<PaymentRequired, X402Error> {
    decode_header(value)
}

/// Convenience — encode a [`PaymentPayload`] for `X-PAYMENT`.
pub fn encode_payment(p: &PaymentPayload) -> Result<String, X402Error> {
    encode_header(p)
}

/// Convenience — decode an `X-PAYMENT` header into a [`PaymentPayload`].
pub fn decode_payment(value: &str) -> Result<PaymentPayload, X402Error> {
    decode_header(value)
}

/// Convenience — encode a [`PaymentReceipt`] for `X-PAYMENT-RESPONSE`.
pub fn encode_receipt(r: &PaymentReceipt) -> Result<String, X402Error> {
    encode_header(r)
}

/// Convenience — decode an `X-PAYMENT-RESPONSE` header into a [`PaymentReceipt`].
pub fn decode_receipt(value: &str) -> Result<PaymentReceipt, X402Error> {
    decode_header(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Network, PaymentSpec};

    #[test]
    fn header_roundtrip() {
        let pr = PaymentRequired::single(
            PaymentSpec::usdc(Network::BaseMainnet, "1000", "0xabc")
                .with_resource("https://example.com"),
        );
        let encoded = encode_payment_required(&pr).unwrap();
        let back = decode_payment_required(&encoded).unwrap();
        assert_eq!(back, pr);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode_payment_required("@@@not-base64@@@").is_err());
        // valid base64, invalid json:
        assert!(decode_payment_required("Zm9v").is_err());
    }
}
