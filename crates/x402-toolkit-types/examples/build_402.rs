//! Build a `PaymentRequired` challenge and dump it as JSON + as the
//! base64-encoded `X-PAYMENT-REQUIRED` header value.
//!
//! Runs offline. No wallet, no network. Useful for sanity-checking the
//! wire format of your spec before you wire up a real server.
//!
//! ```sh
//! cargo run -p x402-toolkit-types --example build_402
//! ```

use x402_toolkit_types::{headers, Network, PaymentRequired, PaymentSpec};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = PaymentSpec::usdc(
        Network::BaseSepolia,
        "1000", // 1000 micro-USDC = $0.001
        "0x9876543210987654321098765432109876543210",
    )
    .with_resource("https://api.example.com/v1/chat")
    .with_description("$0.001 USDC per call");

    let challenge = PaymentRequired::single(spec);

    println!("--- JSON body ---");
    println!("{}", serde_json::to_string_pretty(&challenge)?);

    println!("\n--- X-PAYMENT-REQUIRED header value (base64) ---");
    println!("{}", headers::encode_payment_required(&challenge)?);

    Ok(())
}
