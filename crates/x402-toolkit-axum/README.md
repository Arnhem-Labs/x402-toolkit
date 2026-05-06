# x402-toolkit-axum

[![Crates.io](https://img.shields.io/crates/v/x402-toolkit-axum.svg)](https://crates.io/crates/x402-toolkit-axum)
[![Docs](https://docs.rs/x402-toolkit-axum/badge.svg)](https://docs.rs/x402-toolkit-axum)

Axum extractors and ergonomics over [`x402-toolkit-tower`](../x402-toolkit-tower).

```rust,no_run
use axum::{Router, routing::get};
use x402_toolkit_axum::{Receipt, X402Layer, LayerConfig};
use x402_toolkit_client::MockFacilitator;
use x402_toolkit_types::{Network, PaymentSpec};

#[tokio::main]
async fn main() {
    let cfg = LayerConfig::new(
        PaymentSpec::usdc(Network::BaseSepolia, "1000", "0xVAULT")
            .with_resource("https://localhost/api"),
        MockFacilitator::default(),
    );
    let app = Router::new()
        .route("/api", get(handler))
        .layer(X402Layer::new(cfg));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handler(Receipt(r): Receipt) -> String {
    format!("paid by {}", r.payer)
}
```

## License

Dual-licensed under MIT or Apache-2.0.
