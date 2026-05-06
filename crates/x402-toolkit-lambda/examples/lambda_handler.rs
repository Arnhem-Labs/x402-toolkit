//! Minimal lambda handler example.
//!
//! Compiles to an AWS Lambda binary; deploy with `cargo lambda` (see
//! https://www.cargo-lambda.info/). Locally, you can drive it with
//! `cargo lambda watch` and `cargo lambda invoke` against a JSON
//! API-Gateway-v2 fixture.

use axum::{routing::get, Router};
use x402_toolkit_axum::{LayerConfig, Receipt, X402Layer};
use x402_toolkit_client::MockFacilitator;
use x402_toolkit_lambda::{run_with_axum, LambdaError};
use x402_toolkit_types::{Network, PaymentSpec};

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    let cfg = LayerConfig::new(
        PaymentSpec::usdc(
            Network::BaseSepolia,
            "1000",
            "0x9876543210987654321098765432109876543210",
        ),
        MockFacilitator::default(),
    );
    let app = Router::new()
        .route("/api", get(handler))
        .route("/health", get(|| async { "ok" }))
        .layer(X402Layer::new(cfg));
    run_with_axum(app).await
}

async fn handler(Receipt(r): Receipt) -> String {
    format!("paid by {}", r.payer)
}
