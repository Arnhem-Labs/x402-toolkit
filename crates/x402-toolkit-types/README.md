# x402-toolkit-types

[![Crates.io](https://img.shields.io/crates/v/x402-toolkit-types.svg)](https://crates.io/crates/x402-toolkit-types)
[![Docs](https://docs.rs/x402-toolkit-types/badge.svg)](https://docs.rs/x402-toolkit-types)

Pure protocol types for the [x402](https://x402.org) HTTP-native payment
standard. No I/O, no async runtime, no framework dependencies — just the wire
format, EIP-712 hashing for EIP-3009 transfer authorizations, and a few small
helpers.

This is the leaf crate of the [x402-toolkit](https://github.com/Arnhem-Labs/x402-toolkit)
workspace. Every other crate depends on it.

```rust
use x402_toolkit_types::{Network, PaymentSpec, PaymentRequired};

let spec = PaymentSpec::usdc(
    Network::BaseSepolia,
    "1000",                                              // 1000 micro-USDC = $0.001
    "0x9876543210987654321098765432109876543210",        // pay-to vault
)
.with_resource("https://api.example.com/v1/chat")
.with_description("$0.001 USDC per call");

let challenge = PaymentRequired::single(spec);
println!("{}", serde_json::to_string_pretty(&challenge).unwrap());
```

## License

Dual-licensed under MIT or Apache-2.0. See the workspace
[LICENSE-MIT](../../LICENSE-MIT) / [LICENSE-APACHE](../../LICENSE-APACHE).
