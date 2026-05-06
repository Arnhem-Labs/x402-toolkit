//! Core protocol types for [x402](https://x402.org), the HTTP-native
//! programmatic payments standard.
//!
//! This crate is **leaf-clean**: no I/O, no async runtime, no framework
//! dependencies. It contains the wire-format types, EIP-3009 / EIP-712
//! hashing helpers, and small (de)serialization utilities used by every
//! other crate in the [x402-toolkit] workspace.
//!
//! [x402-toolkit]: https://github.com/Arnhem-Labs/x402-toolkit
//!
//! # What's here
//!
//! - [`PaymentSpec`] / [`PaymentRequired`] — the server-side challenge
//!   returned with HTTP `402 Payment Required`.
//! - [`PaymentPayload`] — the client-side `X-PAYMENT` header contents.
//! - [`PaymentReceipt`] — the server-side `X-PAYMENT-RESPONSE` header /
//!   facilitator settlement output.
//! - [`Network`] — open enum for EVM L1/L2s with a `Custom` escape hatch.
//! - [`eip3009::TransferWithAuthorization`] — EIP-3009 typed data with an
//!   [`eip712_hash`](eip3009::TransferWithAuthorization::eip712_hash) helper.
//! - [`X402Error`] — non-async error type for protocol-level failures.
//!
//! # Quick example
//!
//! ```
//! use x402_toolkit_types::{Network, PaymentSpec, PaymentRequired};
//!
//! let spec = PaymentSpec::usdc(
//!     Network::BaseSepolia,
//!     "1000",
//!     "0x9876543210987654321098765432109876543210",
//! );
//! let challenge = PaymentRequired::single(spec);
//! let json = serde_json::to_string(&challenge).unwrap();
//! assert!(json.contains("\"version\":\"2\""));
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod eip3009;
pub mod error;
pub mod headers;
pub mod network;
pub mod payload;
pub mod receipt;
pub mod spec;

pub use error::X402Error;
pub use network::Network;
pub use payload::{Authorization, PaymentPayload};
pub use receipt::PaymentReceipt;
pub use spec::{PaymentRequired, PaymentSpec, Scheme};

/// Mainnet USDC contract on Base (`eip155:8453`).
///
/// USDC on Base has 6 decimals — `"1000000"` units = 1 USDC.
pub const USDC_BASE_MAINNET: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";

/// USDC contract on Base Sepolia testnet (`eip155:84532`).
pub const USDC_BASE_SEPOLIA: &str = "0x036CbD53842c5426634e7929541eC2318f3dCF7e";

/// The x402 protocol version this crate implements (V2 / December 2025).
pub const X402_VERSION: &str = "2";
