//! Axum-flavored ergonomics for the x402-toolkit middleware.
//!
//! Most of the heavy lifting is in [`x402_toolkit_tower`]; this crate is
//! a thin re-export layer plus an axum [`Receipt`] extractor.
//!
//! # Example
//!
//! ```no_run
//! use axum::{Router, routing::get};
//! use x402_toolkit_axum::{LayerConfig, Receipt, X402Layer};
//! use x402_toolkit_client::MockFacilitator;
//! use x402_toolkit_types::{Network, PaymentSpec};
//!
//! async fn handler(Receipt(r): Receipt) -> String {
//!     format!("paid by {}", r.payer)
//! }
//!
//! # async fn run() {
//! let cfg = LayerConfig::new(
//!     PaymentSpec::usdc(Network::BaseSepolia, "1000", "0x9876543210987654321098765432109876543210")
//!         .with_resource("http://localhost/api"),
//!     MockFacilitator::default(),
//! );
//! let app: Router = Router::new()
//!     .route("/api", get(handler))
//!     .layer(X402Layer::new(cfg));
//! # let _ = app;
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod extractor;

pub use extractor::{OptionalReceipt, Receipt};
pub use x402_toolkit_tower::{
    build_402_response, InMemoryStore, LayerConfig, ReceiptStore, StoreError, X402Layer,
    X402Service,
};
