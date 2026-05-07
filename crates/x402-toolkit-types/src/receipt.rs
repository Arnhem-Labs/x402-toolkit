//! `PaymentReceipt` — the decoded contents of `X-PAYMENT-RESPONSE`.
//!
//! On success, the server returns a `200` (or whatever the gated endpoint
//! would normally return) plus an `X-PAYMENT-RESPONSE` header containing a
//! base64-encoded JSON [`PaymentReceipt`]. Clients can use it to confirm
//! the on-chain settlement that paid for the request.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Network;

/// Server-side receipt for a successful payment.
///
/// # Wire format
///
/// ```json
/// {
///   "success": true,
///   "transaction": "0x...txHash",
///   "network": "eip155:8453",
///   "payer": "0x...payer"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentReceipt {
    /// `true` if the facilitator confirmed settlement. `false` should not
    /// normally appear in an `X-PAYMENT-RESPONSE` header (the server should
    /// return `402` instead) — it's allowed by the spec for symmetry.
    pub success: bool,
    /// On-chain transaction hash for the settlement, when available.
    /// Optional because some facilitators batch settlements off-chain.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transaction: Option<String>,
    /// Network the payment settled on.
    pub network: Network,
    /// EVM address of the payer.
    pub payer: String,
    /// Optional UTC timestamp of when the facilitator verified the payment.
    /// Populated by `x402-toolkit-tower`'s middleware on its way back to
    /// the client.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub verified_at: Option<DateTime<Utc>>,
}

impl PaymentReceipt {
    /// Build a successful receipt.
    pub fn ok(network: Network, payer: impl Into<String>) -> Self {
        Self {
            success: true,
            transaction: None,
            network,
            payer: payer.into(),
            verified_at: Some(Utc::now()),
        }
    }

    /// Attach an on-chain transaction hash.
    pub fn with_transaction(mut self, tx: impl Into<String>) -> Self {
        self.transaction = Some(tx.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_none_fields() {
        let r = PaymentReceipt {
            success: true,
            transaction: None,
            network: Network::BaseMainnet,
            payer: "0xabc".into(),
            verified_at: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert!(v.get("transaction").is_none());
        assert!(v.get("verified_at").is_none());
        assert_eq!(v["success"], true);
        assert_eq!(v["network"], "eip155:8453");
    }

    #[test]
    fn roundtrip() {
        let r = PaymentReceipt::ok(Network::BaseSepolia, "0xabc").with_transaction("0xtx");
        let json = serde_json::to_string(&r).unwrap();
        let back: PaymentReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
}
