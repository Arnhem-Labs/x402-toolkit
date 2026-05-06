# Contributing to x402-toolkit

Thanks for your interest. The toolkit is a small, opinionated set of crates;
contributions that align with the existing direction are welcome.

## Ground rules

- Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace` before opening a PR. CI enforces all three.
- New public APIs need rustdoc with a runnable `# Examples` section.
- The four cloud-neutral crates (`-types`, `-client`, `-tower`, `-axum`,
  `-cli`) must stay free of AWS / single-vendor dependencies. CI's
  agnosticism audit will fail your PR if you leak an `aws_*` symbol or a
  hard `coinbase` reference outside `x402-toolkit-client::facilitator::cdp`.

## Reporting security issues

See [SECURITY.md](SECURITY.md). Do **not** open public issues for
vulnerabilities.

## License

By submitting a PR you agree that your contribution is dual-licensed under
MIT and Apache-2.0, per the workspace `LICENSE-*` files.
