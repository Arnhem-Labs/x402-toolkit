//! `x402t probe <url>` — GET a URL and dump the 402 challenge.

use clap::Args;
use reqwest::header::HeaderMap;

use x402_toolkit_types::{headers, PaymentRequired};

#[derive(Debug, Args)]
pub struct ProbeArgs {
    /// URL of the gated resource.
    pub url: String,
}

pub async fn run(args: ProbeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::Client::new().get(&args.url).send().await?;

    if resp.status().as_u16() != 402 {
        eprintln!("warning: expected 402, got {}", resp.status());
    }

    let pr = challenge_from_headers(resp.headers())?
        .or_else(|| {
            // Some servers may not set X-PAYMENT-REQUIRED but put the
            // challenge in the body; try parsing the body if it's JSON.
            None
        });

    let pr = match pr {
        Some(p) => p,
        None => {
            // Fall back to body-as-JSON.
            let bytes = resp.bytes().await?;
            serde_json::from_slice::<PaymentRequired>(&bytes)
                .map_err(|e| format!("no X-PAYMENT-REQUIRED header and body isn't a PaymentRequired: {e}"))?
        }
    };

    let json = serde_json::to_string_pretty(&pr)?;
    println!("{json}");
    Ok(())
}

fn challenge_from_headers(h: &HeaderMap) -> Result<Option<PaymentRequired>, Box<dyn std::error::Error>> {
    let Some(v) = h.get(headers::X_PAYMENT_REQUIRED) else {
        return Ok(None);
    };
    let s = v.to_str()?;
    Ok(Some(headers::decode_payment_required(s)?))
}
