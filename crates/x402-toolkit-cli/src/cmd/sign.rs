//! `x402t sign` — produce a signed X-PAYMENT header for a given spec.

use std::path::PathBuf;

use clap::Args;
use x402_toolkit_client::{LocalSigner, PaymentBuilder};
use x402_toolkit_types::{headers, PaymentRequired, PaymentSpec};

#[derive(Debug, Args)]
pub struct SignArgs {
    /// Path to a JSON file containing either a `PaymentRequired`
    /// challenge or a single `PaymentSpec`. Use `-` to read from stdin.
    #[arg(long)]
    pub spec: String,

    /// 32-byte hex private key (with or without `0x` prefix). Use the
    /// `X402T_PRIVATE_KEY` env var if you'd rather not pass keys on
    /// the command line.
    #[arg(long, env = "X402T_PRIVATE_KEY")]
    pub key: String,

    /// Optional 0..N index into the `accepts` array if the spec is a
    /// `PaymentRequired` (default: 0).
    #[arg(long, default_value_t = 0)]
    pub index: usize,
}

pub fn run(args: SignArgs) -> Result<(), Box<dyn std::error::Error>> {
    let raw = read_input(&args.spec)?;
    let spec = parse_spec(&raw, args.index)?;

    let signer = LocalSigner::from_hex(&args.key)?;
    // We're inside an async runtime — spawn the sign call.
    let payload = futures::executor::block_on(PaymentBuilder::for_spec(&spec).sign(&signer))?;

    let header = headers::encode_payment(&payload)?;
    println!("{header}");
    Ok(())
}

fn read_input(arg: &str) -> Result<String, Box<dyn std::error::Error>> {
    if arg == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s)
    } else {
        let path: PathBuf = arg.into();
        Ok(std::fs::read_to_string(path)?)
    }
}

fn parse_spec(raw: &str, index: usize) -> Result<PaymentSpec, Box<dyn std::error::Error>> {
    if let Ok(pr) = serde_json::from_str::<PaymentRequired>(raw) {
        return pr
            .accepts
            .into_iter()
            .nth(index)
            .ok_or_else(|| format!("PaymentRequired had no accepts[{index}]").into());
    }
    let s: PaymentSpec = serde_json::from_str(raw)?;
    Ok(s)
}
