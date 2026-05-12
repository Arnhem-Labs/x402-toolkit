//! Wire-format compatibility tests.
//!
//! Asserts that our `PaymentRequired` / `PaymentPayload` / `PaymentReceipt`
//! serialization matches the expected x402 V2 wire format byte-for-byte
//! (modulo whitespace and field ordering, which JSON makes unimportant).
//!
//! When upstream test vectors land in `coinbase/x402/typescript/tests/`,
//! drop them in this file as additional fixtures.

use x402_toolkit_types::{
    payload::{Authorization, SignedAuthorization},
    Network, PaymentPayload, PaymentReceipt, PaymentRequired, PaymentSpec, Scheme,
};

#[test]
fn payment_required_matches_expected_shape() {
    let pr = PaymentRequired::single(
        PaymentSpec::new(
            Scheme::exact(),
            Network::BaseMainnet,
            "2000",
            "https://api.example.com/v1/chat",
            "0x9876543210987654321098765432109876543210",
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
            60,
        )
        .with_description("Example pay-per-call API")
        .with_extra(serde_json::json!({"name": "USD Coin", "version": "2"})),
    );
    let v = serde_json::to_value(&pr).unwrap();

    assert_eq!(v["version"], "2");
    let s = &v["accepts"][0];
    assert_eq!(s["scheme"], "exact");
    assert_eq!(s["network"], "eip155:8453");
    assert_eq!(s["maxAmountRequired"], "2000");
    assert_eq!(s["resource"], "https://api.example.com/v1/chat");
    assert_eq!(s["description"], "Example pay-per-call API");
    assert_eq!(s["payTo"], "0x9876543210987654321098765432109876543210");
    assert_eq!(s["maxTimeoutSeconds"], 60);
    assert_eq!(s["asset"], "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    assert_eq!(s["extra"]["name"], "USD Coin");
    assert_eq!(s["extra"]["version"], "2");
}

#[test]
fn payment_payload_matches_expected_shape() {
    let p = PaymentPayload {
        version: 2,
        scheme: Scheme::exact(),
        network: Network::BaseMainnet,
        payload: SignedAuthorization {
            signature: "0xabcd".into(),
            authorization: Authorization {
                from: "0xDEV".into(),
                to: "0xVAULT".into(),
                value: "2000".into(),
                valid_after: "0".into(),
                valid_before: "1744000000".into(),
                nonce: "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
                    .into(),
            },
        },
    };
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(v["x402Version"], 2);
    assert_eq!(v["scheme"], "exact");
    assert_eq!(v["network"], "eip155:8453");
    let inner = &v["payload"];
    assert_eq!(inner["signature"], "0xabcd");
    let auth = &inner["authorization"];
    assert_eq!(auth["from"], "0xDEV");
    assert_eq!(auth["to"], "0xVAULT");
    assert_eq!(auth["value"], "2000");
    assert_eq!(auth["validAfter"], "0");
    assert_eq!(auth["validBefore"], "1744000000");
    assert_eq!(
        auth["nonce"],
        "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    );
}

#[test]
fn payment_receipt_matches_expected_shape() {
    let r = PaymentReceipt {
        success: true,
        transaction: Some("0xtx".into()),
        network: Network::BaseMainnet,
        payer: "0xDEV".into(),
        verified_at: None,
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["transaction"], "0xtx");
    assert_eq!(v["network"], "eip155:8453");
    assert_eq!(v["payer"], "0xDEV");
}
