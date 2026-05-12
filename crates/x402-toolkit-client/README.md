# x402-toolkit-client

[![Crates.io](https://img.shields.io/crates/v/x402-toolkit-client.svg)](https://crates.io/crates/x402-toolkit-client)
[![Docs](https://docs.rs/x402-toolkit-client/badge.svg)](https://docs.rs/x402-toolkit-client)

Async HTTP client, [`WalletSigner`] trait, and pluggable [`Facilitator`]
trait for the [x402](https://x402.org) payment protocol.

```rust,no_run
use x402_toolkit_client::{LocalSigner, MockFacilitator, X402Client};
use x402_toolkit_types::{Network, PaymentSpec};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let signer = LocalSigner::random();
let facilitator = MockFacilitator::default();

let spec = PaymentSpec::usdc(Network::BaseSepolia, "1000", "0xVAULT");
let payload = x402_toolkit_client::sign_authorization(&signer, &spec).await?;
let receipt = facilitator.verify(&payload).await?;
println!("verified: payer = {}", receipt.payer);
# Ok(()) }
```

## v0.1 Status

- ✅ `LocalSigner` real EIP-3009 signing via `alloy-signer-local`.
- ✅ `MockFacilitator` for hermetic local-dev / tests.
- ⚠️ `CdpFacilitator` and `HttpFacilitator` are real HTTP clients but not
  yet exercised against a live Coinbase CDP or self-hosted facilitator in
  CI — see the `sepolia-example` feature for a Sepolia round-trip.
- ⚠️ Tokio-only async runtime in v0.1. Runtime-agnostic transport is a
  v0.2 epic.

## License

Dual-licensed under MIT or Apache-2.0.
