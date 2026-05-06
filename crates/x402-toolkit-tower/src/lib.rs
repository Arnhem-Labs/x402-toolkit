//! Framework-agnostic [`tower::Layer`] for gating HTTP services on x402
//! payments.
//!
//! Wrap any `tower::Service<http::Request<B>>` with [`X402Layer`] and:
//!
//! 1. Requests without an `X-PAYMENT` header get back `402 Payment
//!    Required` with an `X-PAYMENT-REQUIRED` challenge built from the
//!    layer's [`LayerConfig`].
//! 2. Requests with an `X-PAYMENT` header are decoded, sent to the
//!    configured [`Facilitator`] for verification, and — on success —
//!    forwarded to the inner service with the [`PaymentReceipt`]
//!    attached as a request extension. The middleware also adds the
//!    `X-PAYMENT-RESPONSE` header to the inner service's response.
//! 3. Replays are caught by an optional [`ReceiptStore`] (default:
//!    in-memory; opt into [`PgReceiptStore`](pg::PgReceiptStore) under
//!    feature `pg-store`).
//!
//! [`Facilitator`]: x402_toolkit_client::Facilitator
//! [`PaymentReceipt`]: x402_toolkit_types::PaymentReceipt

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod extract;
pub mod layer;
pub mod store;

#[cfg(feature = "pg-store")]
#[cfg_attr(docsrs, doc(cfg(feature = "pg-store")))]
pub mod pg;

pub use extract::build_402_response;
pub use layer::{LayerConfig, X402Layer, X402Service};
pub use store::{InMemoryStore, ReceiptStore, StoreError};

#[cfg(feature = "pg-store")]
pub use pg::PgReceiptStore;
