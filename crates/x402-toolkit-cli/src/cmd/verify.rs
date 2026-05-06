//! `x402t verify` — call a remote facilitator with a signed payload.

use clap::Args;
use x402_toolkit_client::{Facilitator, HttpFacilitator};
use x402_toolkit_types::{headers, PaymentPayload};

#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// Base64-encoded `X-PAYMENT` header value. Use `-` to read from stdin.
    #[arg(long)]
    pub payload: String,

    /// Facilitator base URL (e.g. `https://facilitator.coinbase.com` or
    /// `http://localhost:8402`).
    #[arg(long)]
    pub facilitator: String,

    /// Optional bearer token (for Coinbase CDP, `<api_key>`).
    #[arg(long, env = "X402T_BEARER")]
    pub bearer: Option<String>,
}

pub async fn run(args: VerifyArgs) -> Result<(), Box<dyn std::error::Error>> {
    let raw = if args.payload == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        s.trim().to_string()
    } else {
        args.payload.clone()
    };

    let payload: PaymentPayload = headers::decode_payment(&raw)?;
    let mut f = HttpFacilitator::new(args.facilitator);
    if let Some(t) = args.bearer {
        f = f.with_bearer_token(t);
    }
    let receipt = f.verify(&payload).await?;
    let json = serde_json::to_string_pretty(&receipt)?;
    println!("{json}");
    Ok(())
}
