# x402-toolkit

A Rust-native toolkit for [x402](https://x402.org), the open standard for HTTP-native programmatic payments. Build payment-gated APIs, MCP servers, and usage-based-billing services in Axum; consume them from Rust agents and bots without API keys, accounts, or sessions — just a wallet.

**Six crates that compose. Plug in your runtime, signer, store, and facilitator.**

```rust
use axum::{Router, routing::get};
use x402_toolkit_axum::{X402Layer, Receipt};
use x402_toolkit_client::MockFacilitator;
use x402_toolkit_types::{PaymentSpec, Network};

#[tokio::main]
async fn main() {
    let spec = PaymentSpec::usdc(Network::BaseSepolia, "1000", "0xVAULT");
    let app = Router::new()
        .route("/pro", get(|Receipt(r): Receipt| async move { format!("paid by {}", r.payer) }))
        .layer(X402Layer::new(spec, MockFacilitator::default()));
    axum::serve(tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(), app)
        .await.unwrap();
}
```

## Crates

| Crate | Role |
|---|---|
| [`x402-toolkit-types`](crates/x402-toolkit-types) | Pure protocol types, EIP-3009 helpers, no I/O, no async |
| [`x402-toolkit-client`](crates/x402-toolkit-client) | Async HTTP client + `WalletSigner` / `Facilitator` traits |
| [`x402-toolkit-tower`](crates/x402-toolkit-tower) | Framework-agnostic `tower::Layer` + `ReceiptStore` |
| [`x402-toolkit-axum`](crates/x402-toolkit-axum) | Axum ergonomics over `x402-toolkit-tower` |
| [`x402-toolkit-lambda`](crates/x402-toolkit-lambda) | Optional AWS Lambda runtime adapter |
| [`x402-toolkit-cli`](crates/x402-toolkit-cli) | `x402t` — probe, sign, verify, run a mock facilitator |

The crates are runtime-agnostic where it matters: any `tower::Service` works (axum, hyper, actix, tonic, …); any EVM L1/L2 via `Network::Custom`; any EIP-3009 token via `PaymentSpec.asset`; pluggable facilitator (Coinbase CDP, self-hosted, mock); pluggable wallet signer; pluggable receipt store.

## Quickstart

Run the axum demo against an in-process mock facilitator (no wallet, no testnet, no creds):

```sh
cargo run -p x402-toolkit-axum --example axum_server
# then in another shell:
curl -i http://localhost:3000/pro            # 402 + X-PAYMENT-REQUIRED challenge
x402t probe http://localhost:3000/pro        # decodes the challenge
x402t sign --spec '<challenge>' --key 0x...  # produces signed X-PAYMENT header
```

Real Sepolia round-trip (needs `BASE_SEPOLIA_RPC_URL` + `TEST_PRIVATE_KEY`):

```sh
cargo run -p x402-toolkit-client --example pay_request --features sepolia-example
```

## Documentation

- Per-crate READMEs and rustdoc on docs.rs once published.
- Recipes for [axum](docs/src/recipes/axum.md), [Lambda](docs/src/recipes/lambda.md), [signing](docs/src/recipes/signing.md), [Sepolia](docs/src/recipes/sepolia.md).
- [Comparison vs prior art](docs/src/comparison.md) — `x402-rs`, `x402-facilitator`, `x402-paywall`.

## Status

v0.1 ships:

- Real working implementations on every public symbol (no `unimplemented!` panics).
- `MockFacilitator` + `LocalSigner` + `InMemoryStore` give you a fully hermetic test stack.
- `CdpFacilitator` for production against `facilitator.coinbase.com`; `HttpFacilitator` for any compliant endpoint.
- `scheme = "exact"` over EIP-3009 only. Other schemes: planned for later versions.
- Tokio-only async runtime in `x402-toolkit-client`; runtime-agnostic transport is a v0.2 epic.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
