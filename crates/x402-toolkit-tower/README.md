# x402-toolkit-tower

[![Crates.io](https://img.shields.io/crates/v/x402-toolkit-tower.svg)](https://crates.io/crates/x402-toolkit-tower)
[![Docs](https://docs.rs/x402-toolkit-tower/badge.svg)](https://docs.rs/x402-toolkit-tower)

Framework-agnostic [`tower::Layer`](https://docs.rs/tower/latest/tower/layer/trait.Layer.html)
gating any HTTP service on x402 payments. Works with axum, hyper directly,
tonic, poem, salvo, warp — anything that speaks `tower::Service<http::Request>`.

```rust,no_run
use x402_toolkit_tower::{X402Layer, LayerConfig};
use x402_toolkit_client::MockFacilitator;
use x402_toolkit_types::{Network, PaymentSpec};

let cfg = LayerConfig::new(
    PaymentSpec::usdc(Network::BaseSepolia, "1000", "0xVAULT"),
    MockFacilitator::default(),
);
let layer = X402Layer::new(cfg);
// Apply to any tower-compatible router:
//   .layer(layer)
```

## Features

- `default = []`
- `pg-store` — adds `PgReceiptStore` (sqlx-backed) and ships
  `migrations/0001_x402_payment_receipts.sql`.

## License

Dual-licensed under MIT or Apache-2.0.
