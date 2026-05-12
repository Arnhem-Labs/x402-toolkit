# x402-toolkit-cli

[![Crates.io](https://img.shields.io/crates/v/x402-toolkit-cli.svg)](https://crates.io/crates/x402-toolkit-cli)

`x402t` — the developer tool for building, debugging, and exercising x402
payment integrations.

```sh
cargo install x402-toolkit-cli --features mock
```

## Subcommands

```sh
x402t probe <url>                          # GET <url>, dump the 402 challenge
x402t sign --spec <json> --key <hex>       # produce a signed X-PAYMENT header
x402t verify --payload <b64> --facilitator <url> [--bearer <token>]
x402t mock-facilitator --port 8402         # run a local mock (feature `mock`)
```

## Quickstart

```sh
# Terminal 1: run the canonical demo server
cargo run -p x402-toolkit-axum --example axum_server

# Terminal 2: probe + sign + retry
x402t probe http://localhost:3000/api > challenge.json
x402t sign --spec challenge.json \
           --key 0x0000000000000000000000000000000000000000000000000000000000000001 \
           > payment.b64
curl -i -H "X-PAYMENT: $(cat payment.b64)" http://localhost:3000/api
```

## License

Dual-licensed under MIT or Apache-2.0.
