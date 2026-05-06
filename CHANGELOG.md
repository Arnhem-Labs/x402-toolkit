# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Per-crate changelogs are maintained in `crates/<name>/CHANGELOG.md` and updated
automatically by [release-plz](https://release-plz.dev/).

## [Unreleased]

### Added

- Initial six-crate workspace: `x402-toolkit-types`, `x402-toolkit-client`,
  `x402-toolkit-tower`, `x402-toolkit-axum`, `x402-toolkit-lambda`,
  `x402-toolkit-cli`.
- Protocol types and EIP-3009 EIP-712 hashing in `x402-toolkit-types`.
- `Facilitator` trait with `CdpFacilitator`, `HttpFacilitator`, and
  `MockFacilitator` impls in `x402-toolkit-client`.
- `WalletSigner` trait with `LocalSigner` impl wrapping
  `alloy-signer-local`.
- `X402Layer` / `X402Service` tower middleware with `ReceiptStore` trait
  and `InMemoryStore` / `PgReceiptStore` implementations.
- Axum extractor, router extension trait, and `from_fn` helper in
  `x402-toolkit-axum`.
- AWS Lambda adapter (`run_with_layer`) in `x402-toolkit-lambda`.
- `x402t` CLI binary with `probe`, `sign`, `verify`, and `mock-facilitator`
  subcommands.
