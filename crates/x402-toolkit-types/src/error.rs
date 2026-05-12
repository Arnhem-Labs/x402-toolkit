//! Protocol-level error type.

use thiserror::Error;

/// Errors raised while constructing or parsing x402 wire-format messages.
///
/// I/O errors (network, facilitator, runtime) live in
/// `x402_toolkit_client::ClientError`, which wraps this type for protocol
/// failures and adds its own variants for transport problems. Keeping
/// `X402Error` free of `reqwest::Error` keeps `x402-toolkit-types` leaf-clean
/// — no async runtime, no HTTP client.
#[derive(Debug, Error)]
pub enum X402Error {
    /// The supplied input was structurally invalid (missing field, wrong
    /// shape, malformed hex, etc).
    #[error("invalid input: {0}")]
    Invalid(String),

    /// A field had the right shape but a value outside its allowed range
    /// (e.g. a negative amount, a `validBefore` in the past).
    #[error("out of range: {0}")]
    OutOfRange(String),

    /// Base64 decoding of an `X-PAYMENT` / `X-PAYMENT-REQUIRED` header
    /// failed.
    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    /// Hex decoding of an address, signature, or 32-byte nonce failed.
    #[error("hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
