//! Error type for the client crate.

use thiserror::Error;

use x402_toolkit_types::X402Error;

/// Errors raised by the x402 client, signers, and facilitator impls.
///
/// Wraps [`X402Error`] for protocol-level problems and adds variants for
/// transport (`reqwest`), signing, and facilitator-rejected payments.
#[derive(Debug, Error)]
pub enum ClientError {
    /// Protocol-level problem: malformed types, unsupported scheme, etc.
    #[error("protocol: {0}")]
    Protocol(String),

    /// Mirror of [`X402Error::Invalid`].
    #[error("invalid spec: {0}")]
    InvalidSpec(String),

    /// Signing failed (KMS down, key unavailable, …).
    #[error("signer: {0}")]
    Signer(String),

    /// HTTP transport error.
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization failure.
    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),

    /// Facilitator rejected the payment for a protocol reason
    /// (insufficient value, expired signature, replayed nonce, …).
    #[error("facilitator rejected payment: {0}")]
    Rejected(String),

    /// Wrapped [`X402Error`] (base64, hex, JSON, validation problems).
    #[error(transparent)]
    Wire(#[from] X402Error),
}
