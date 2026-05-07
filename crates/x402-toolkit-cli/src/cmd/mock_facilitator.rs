//! `x402t mock-facilitator` — run an in-process facilitator for testing.
//!
//! Behind the `mock` feature so the default `cargo install
//! x402-toolkit-cli` doesn't pull axum + hyper.

use std::net::SocketAddr;

use axum::{routing::post, Json, Router};
use clap::Args;
use serde::Deserialize;
use x402_toolkit_client::{Facilitator, MockFacilitator};
use x402_toolkit_types::{PaymentPayload, PaymentReceipt};

#[derive(Debug, Args)]
pub struct MockArgs {
    /// Port to bind to.
    #[arg(long, default_value_t = 8402)]
    pub port: u16,
}

pub async fn run(args: MockArgs) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new()
        .route("/verify", post(verify_handler))
        .route("/settle", post(settle_handler));

    let addr: SocketAddr = ([127, 0, 0, 1], args.port).into();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("x402t mock-facilitator listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct VerifyBody {
    payload: PaymentPayload,
}

async fn verify_handler(
    Json(body): Json<VerifyBody>,
) -> Result<Json<PaymentReceipt>, (axum::http::StatusCode, String)> {
    let f = MockFacilitator::default();
    f.verify(&body.payload)
        .await
        .map(Json)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))
}

async fn settle_handler(
    body: Json<VerifyBody>,
) -> Result<Json<PaymentReceipt>, (axum::http::StatusCode, String)> {
    verify_handler(body).await
}
