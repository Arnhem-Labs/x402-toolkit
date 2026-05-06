//! Canonical end-to-end x402-toolkit demo.
//!
//! Runs an axum server with one gated route, backed by `MockFacilitator`
//! (in-process, no Coinbase / Sepolia setup). To exercise:
//!
//! ```sh
//! cargo run -p x402-toolkit-axum --example axum_server &
//! curl -i http://localhost:3000/api          # → 402 + X-PAYMENT-REQUIRED
//! ```
//!
//! Use the `x402t sign` CLI (or the `x402-toolkit-client` crate) to
//! generate a signed `X-PAYMENT` header and retry the request.

use axum::{routing::get, Router};
use x402_toolkit_axum::{LayerConfig, Receipt, X402Layer};
use x402_toolkit_client::MockFacilitator;
use x402_toolkit_types::{Network, PaymentSpec};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Build a payment spec: $0.001 USDC on Base Sepolia per request.
    let spec = PaymentSpec::usdc(
        Network::BaseSepolia,
        "1000",
        "0x9876543210987654321098765432109876543210",
    )
    .with_resource("http://localhost:3000/api")
    .with_description("Example pay-per-call API");

    let cfg = LayerConfig::new(spec, MockFacilitator::default());

    // Gated routes go through `X402Layer`; ungated routes (health
    // checks, free GETs) live on the outer router.
    let gated = Router::new()
        .route("/api", get(handler))
        .layer(X402Layer::new(cfg));

    let app = Router::new()
        .merge(gated)
        .route("/health", get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on http://localhost:3000  (try GET /api → 402, GET /health → 200)");
    axum::serve(listener, app).await.unwrap();
}

async fn handler(Receipt(r): Receipt) -> String {
    format!("paid by {} on {}\n", r.payer, r.network.caip2())
}
