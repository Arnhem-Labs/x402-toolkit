# x402-toolkit-lambda

[![Crates.io](https://img.shields.io/crates/v/x402-toolkit-lambda.svg)](https://crates.io/crates/x402-toolkit-lambda)
[![Docs](https://docs.rs/x402-toolkit-lambda/badge.svg)](https://docs.rs/x402-toolkit-lambda)

**Optional** AWS Lambda runtime adapter for `x402-toolkit-tower` /
`x402-toolkit-axum` services.

> If you're not deploying to AWS Lambda, you don't need this crate.
> Use [`x402-toolkit-axum`](../x402-toolkit-axum) directly with
> `axum::serve` (or `hyper::server` for non-axum stacks). This crate
> exists for the AWS-Lambda-on-API-Gateway-v2 deployment path and
> nothing else.

## Quickstart

```rust,no_run
use axum::{Router, routing::get};
use x402_toolkit_axum::{LayerConfig, X402Layer};
use x402_toolkit_client::MockFacilitator;
use x402_toolkit_lambda::run_with_axum;
use x402_toolkit_types::{Network, PaymentSpec};

# async fn _example() -> Result<(), lambda_runtime::Error> {
let cfg = LayerConfig::new(
    PaymentSpec::usdc(Network::BaseSepolia, "1000", "0xVAULT"),
    MockFacilitator::default(),
);
let app = Router::new()
    .route("/api", get(|| async { "paid!" }))
    .layer(X402Layer::new(cfg));

run_with_axum(app).await
# }
```

## License

Dual-licensed under MIT or Apache-2.0.
