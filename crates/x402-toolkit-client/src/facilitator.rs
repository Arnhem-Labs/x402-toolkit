//! [`Facilitator`] trait and three impls: [`CdpFacilitator`],
//! [`HttpFacilitator`], [`MockFacilitator`].
//!
//! The facilitator's job is to turn a signed [`PaymentPayload`] into an
//! on-chain settlement (or, for `MockFacilitator`, a synthetic one) and
//! return a [`PaymentReceipt`] proving it.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use x402_toolkit_types::{eip3009::TransferWithAuthorization, PaymentPayload, PaymentReceipt};

use crate::ClientError;

/// A reason a facilitator declined to settle a payment. Returned with a
/// `400` from real facilitators; surfaced as [`ClientError::Rejected`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentRejection {
    /// Short machine code, e.g. `"insufficient_value"`, `"expired"`,
    /// `"nonce_replayed"`.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

/// Verify (and, where applicable, settle) signed [`PaymentPayload`]s.
///
/// Implementations are expected to be cheap to clone (`Arc<...>`); the
/// `&self` signature lets them be shared across async tasks and stored
/// in a tower middleware's per-service state.
#[async_trait]
pub trait Facilitator: Send + Sync + 'static {
    /// Verify a signed payment. On success, return a receipt; on
    /// protocol-level rejection (bad signature, replayed nonce, expired,
    /// underpayment) return [`ClientError::Rejected`]; on transport
    /// problems return [`ClientError::Http`].
    async fn verify(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError>;

    /// Settle a verified payment on-chain. Default impl re-runs
    /// `verify` (which for `CdpFacilitator` performs settle in the same
    /// HTTP call). Override if your facilitator's verify and settle are
    /// separate operations.
    async fn settle(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError> {
        self.verify(payload).await
    }
}

// =============================================================================
// MockFacilitator
// =============================================================================

/// In-process facilitator that recovers the payer from the signature,
/// validates basic fields, and returns a deterministic synthetic receipt.
///
/// Used by:
/// - Unit / integration tests in this and other crates.
/// - The `x402t mock-facilitator` CLI subcommand (an HTTP wrapper around
///   this type).
/// - The `axum_server` example so it works with no Coinbase / Sepolia
///   set-up.
///
/// **Not a real facilitator.** No on-chain settlement happens. Use only
/// for development and testing.
#[derive(Debug, Clone, Default)]
pub struct MockFacilitator {
    /// If set, every `verify` returns this rejection. Lets tests
    /// exercise rejection paths without crafting a malformed payload.
    pub force_rejection: Option<PaymentRejection>,
}

#[async_trait]
impl Facilitator for MockFacilitator {
    async fn verify(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError> {
        if let Some(r) = &self.force_rejection {
            return Err(ClientError::Rejected(format!("{}: {}", r.code, r.message)));
        }
        verify_signature_recovery(payload)?;
        let auth = &payload.payload.authorization;
        // Deterministic synthetic tx hash: keccak256(nonce || payer)
        let mut input = Vec::with_capacity(32 + auth.from.len());
        input.extend_from_slice(&auth.nonce_bytes()?);
        input.extend_from_slice(auth.from.as_bytes());
        let tx_hash = format!("0x{}", hex::encode(alloy_primitives::keccak256(&input).0));

        Ok(PaymentReceipt::ok(payload.network.clone(), &auth.from).with_transaction(tx_hash))
    }
}

/// Recover the signer from `payload.payload.signature` and assert it
/// matches `payload.payload.authorization.from`.
///
/// `MockFacilitator` doesn't see the original [`PaymentSpec`], so it
/// assumes the canonical USDC EIP-712 domain (`name = "USD Coin"`,
/// `version = "2"`) and the network's canonical USDC address as the
/// verifying contract. This is consistent with payloads produced by
/// `PaymentSpec::usdc(...)`. For non-USDC schemes a real facilitator
/// (`HttpFacilitator` / `CdpFacilitator`) is required.
fn verify_signature_recovery(payload: &PaymentPayload) -> Result<(), ClientError> {
    use alloy_primitives::PrimitiveSignature as Signature;
    use std::str::FromStr;

    let auth = &payload.payload.authorization;
    let twa = TransferWithAuthorization::from_wire(auth)?;
    let verifying = payload.network.usdc_address().ok_or_else(|| {
        ClientError::Protocol("MockFacilitator: network has no canonical USDC address".into())
    })?;
    let digest = twa.eip712_hash("USD Coin", "2", &payload.network, verifying)?;

    let sig_hex = payload
        .payload
        .signature
        .strip_prefix("0x")
        .unwrap_or(&payload.payload.signature);
    let sig_bytes = hex::decode(sig_hex)
        .map_err(|e| ClientError::Protocol(format!("bad signature hex: {e}")))?;
    let sig = Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| ClientError::Protocol(format!("bad signature: {e}")))?;

    let recovered = sig
        .recover_address_from_prehash(&alloy_primitives::B256::from_slice(&digest))
        .map_err(|e| ClientError::Protocol(format!("recover failed: {e}")))?;

    let from_addr = alloy_primitives::Address::from_str(&auth.from)
        .map_err(|e| ClientError::Protocol(format!("bad from address: {e}")))?;

    if recovered != from_addr {
        return Err(ClientError::Rejected(format!(
            "signature recovery: signed by {recovered}, expected {from_addr}"
        )));
    }
    Ok(())
}

// =============================================================================
// HttpFacilitator
// =============================================================================

/// Generic HTTP facilitator. Calls `POST <base>/verify` and
/// `POST <base>/settle` with `{ "payload": <PaymentPayload> }` bodies and
/// expects `{ "success": true, "transaction": "0x...", "payer": "0x...",
/// "network": "eip155:..." }` back.
///
/// Designed for self-hosted facilitators and forks of the Coinbase CDP
/// wire format. For Coinbase CDP itself, prefer [`CdpFacilitator`] which
/// wires up the bearer token.
#[derive(Debug, Clone)]
pub struct HttpFacilitator {
    /// Base URL, e.g. `"https://facilitator.example.com"`.
    pub base_url: String,
    /// Optional bearer token. Set if your facilitator authenticates.
    pub bearer_token: Option<String>,
    /// Underlying HTTP client.
    pub client: reqwest::Client,
}

impl HttpFacilitator {
    /// Build a new `HttpFacilitator` against the given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            bearer_token: None,
            client: reqwest::Client::new(),
        }
    }

    /// Attach a bearer token (e.g. `Bearer <api_key>`).
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    async fn call(
        &self,
        path: &str,
        payload: &PaymentPayload,
    ) -> Result<PaymentReceipt, ClientError> {
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let mut req = self
            .client
            .post(url)
            .json(&serde_json::json!({ "payload": payload }));
        if let Some(t) = &self.bearer_token {
            req = req.bearer_auth(t);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return if status.as_u16() == 400 || status.as_u16() == 402 {
                Err(ClientError::Rejected(body))
            } else {
                Err(ClientError::Protocol(format!(
                    "facilitator {status}: {body}"
                )))
            };
        }
        let receipt: PaymentReceipt = resp.json().await?;
        Ok(receipt)
    }
}

#[async_trait]
impl Facilitator for HttpFacilitator {
    async fn verify(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError> {
        self.call("verify", payload).await
    }
    async fn settle(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError> {
        self.call("settle", payload).await
    }
}

// =============================================================================
// CdpFacilitator
// =============================================================================

/// Coinbase CDP facilitator. Hits `https://facilitator.coinbase.com/...`
/// with the configured CDP API key as a bearer token.
///
/// **Note:** the CDP wire format may evolve; this impl targets the V2
/// spec as of December 2025. If Coinbase ships a breaking change, the
/// `HttpFacilitator` escape hatch lets you point at a compatibility shim
/// while you wait for a `x402-toolkit-client` patch release.
#[derive(Debug, Clone)]
pub struct CdpFacilitator {
    inner: HttpFacilitator,
}

impl CdpFacilitator {
    /// Build a new `CdpFacilitator` with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            inner: HttpFacilitator::new("https://facilitator.coinbase.com")
                .with_bearer_token(api_key),
        }
    }
}

#[async_trait]
impl Facilitator for CdpFacilitator {
    async fn verify(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError> {
        self.inner.verify(payload).await
    }
    async fn settle(&self, payload: &PaymentPayload) -> Result<PaymentReceipt, ClientError> {
        self.inner.settle(payload).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sign_authorization, LocalSigner, WalletSigner};
    use x402_toolkit_types::{Network, PaymentSpec};

    fn spec() -> PaymentSpec {
        PaymentSpec::usdc(
            Network::BaseSepolia,
            "1000",
            "0x9876543210987654321098765432109876543210",
        )
        .with_resource("https://example.com")
    }

    #[tokio::test]
    async fn mock_facilitator_verifies_real_signature() {
        let signer = LocalSigner::random();
        let payload = sign_authorization(&signer, &spec()).await.unwrap();
        let receipt = MockFacilitator::default().verify(&payload).await.unwrap();
        assert!(receipt.success);
        assert_eq!(
            receipt.payer.to_lowercase(),
            signer.address().to_lowercase()
        );
        assert!(receipt.transaction.is_some());
    }

    #[tokio::test]
    async fn mock_facilitator_rejects_tampered_signature() {
        let signer = LocalSigner::random();
        let mut payload = sign_authorization(&signer, &spec()).await.unwrap();
        // Flip a bit in the signature.
        let mut bytes = hex::decode(payload.payload.signature.trim_start_matches("0x")).unwrap();
        bytes[5] ^= 0xff;
        payload.payload.signature = format!("0x{}", hex::encode(bytes));

        let err = MockFacilitator::default()
            .verify(&payload)
            .await
            .unwrap_err();
        match err {
            ClientError::Rejected(_) | ClientError::Protocol(_) => {}
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mock_facilitator_force_rejection_works() {
        let signer = LocalSigner::random();
        let payload = sign_authorization(&signer, &spec()).await.unwrap();
        let f = MockFacilitator {
            force_rejection: Some(PaymentRejection {
                code: "test".into(),
                message: "forced".into(),
            }),
        };
        assert!(matches!(
            f.verify(&payload).await.unwrap_err(),
            ClientError::Rejected(s) if s.contains("forced")
        ));
    }
}
