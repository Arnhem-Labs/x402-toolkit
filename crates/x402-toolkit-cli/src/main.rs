//! `x402t` — debug and develop x402 payment integrations.
//!
//! See `x402t --help` for usage.

use clap::{Parser, Subcommand};

mod cmd;

/// `x402t` — the developer tool for building x402 payment integrations.
#[derive(Debug, Parser)]
#[command(name = "x402t", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// GET a URL and dump the 402 `PaymentRequired` challenge.
    Probe(cmd::probe::ProbeArgs),
    /// Produce a signed `X-PAYMENT` header value for a given spec + key.
    Sign(cmd::sign::SignArgs),
    /// Verify an `X-PAYMENT` header against a remote facilitator.
    Verify(cmd::verify::VerifyArgs),
    /// Run a mock facilitator locally for testing. Requires the `mock`
    /// build feature.
    #[cfg(feature = "mock")]
    MockFacilitator(cmd::mock_facilitator::MockArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    init_tracing();
    if let Err(e) = run().await {
        eprintln!("x402t: error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Probe(a) => cmd::probe::run(a).await,
        Commands::Sign(a) => cmd::sign::run(a),
        Commands::Verify(a) => cmd::verify::run(a).await,
        #[cfg(feature = "mock")]
        Commands::MockFacilitator(a) => cmd::mock_facilitator::run(a).await,
    }
}

fn init_tracing() {
    // Quiet by default; set RUST_LOG=debug or =x402_toolkit_cli=debug
    // to see the wire format printed for each subcommand.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
