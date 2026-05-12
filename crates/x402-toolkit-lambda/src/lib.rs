//! Optional AWS Lambda runtime adapter for `x402-toolkit-tower` /
//! `x402-toolkit-axum` services.
//!
//! **You don't need this crate** unless you're deploying to AWS Lambda
//! behind API Gateway v2. For everything else, use `x402-toolkit-axum`
//! directly with `axum::serve` (or `hyper::server` for non-axum
//! stacks).
//!
//! # Quickstart
//!
//! Build an `axum::Router`, attach `X402Layer` to whichever routes you
//! want gated, and hand the whole router to [`run_with_axum`]:
//!
//! ```no_run
//! use axum::{Router, routing::get};
//! use x402_toolkit_axum::{LayerConfig, X402Layer};
//! use x402_toolkit_client::MockFacilitator;
//! use x402_toolkit_lambda::run_with_axum;
//! use x402_toolkit_types::{Network, PaymentSpec};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), lambda_runtime::Error> {
//!     let cfg = LayerConfig::new(
//!         PaymentSpec::usdc(Network::BaseSepolia, "1000", "0x9876543210987654321098765432109876543210"),
//!         MockFacilitator::default(),
//!     );
//!     let app = Router::new()
//!         .route("/api", get(|| async { "paid!" }))
//!         .layer(X402Layer::new(cfg));
//!     run_with_axum(app).await
//! }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

#[cfg(feature = "with-axum")]
mod axum_adapter;

#[cfg(feature = "with-axum")]
pub use axum_adapter::run_with_axum;

/// Re-export of the underlying lambda_http error type so callers don't
/// need to take a direct dep on `lambda_http` for the success path.
pub use lambda_http::Error as LambdaError;
