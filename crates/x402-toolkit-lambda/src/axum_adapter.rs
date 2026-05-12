//! Adapter for an `axum::Router` running on AWS Lambda + API Gateway v2.

use lambda_http::{run, Error};

/// Run an `axum::Router` on AWS Lambda.
///
/// This is a thin wrapper around [`lambda_http::run`] that exists to:
///
/// 1. Centralize the conversion the toolkit's example code relies on
///    (so we can adjust if `lambda_http` ships a breaking change).
/// 2. Hide the boilerplate `lambda_http::run` requires (the
///    `tower::Service<lambda_http::Request>` adapter).
///
/// Apply your `X402Layer` to the router *before* calling this; the
/// middleware will see all requests that reach the Lambda.
pub async fn run_with_axum(router: axum::Router) -> Result<(), Error> {
    // axum::Router implements tower::Service<lambda_http::Request> via
    // the same trait impls that make it work with hyper. lambda_http
    // delivers a `Request<lambda_http::Body>` and expects a `Response`
    // with a body that can be turned into bytes; axum's body type
    // satisfies this.
    run(router).await
}
