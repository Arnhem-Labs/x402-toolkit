//! Async HTTP client, [`WalletSigner`] trait, and pluggable
//! [`Facilitator`] trait for the [x402](https://x402.org) payment protocol.
//!
//! [`WalletSigner`]: crate::signer::WalletSigner
//! [`Facilitator`]: crate::facilitator::Facilitator
//!
//! # The pieces
//!
//! - [`signer::WalletSigner`] — async trait for "given a 32-byte digest,
//!   return a 65-byte secp256k1 signature." [`signer::LocalSigner`] wraps
//!   `alloy-signer-local` for a hex-key happy path; AWS KMS / Ledger /
//!   browser-injected wallets implement the trait themselves.
//! - [`facilitator::Facilitator`] — async trait for verifying and settling
//!   signed [`PaymentPayload`]s. v0.1 ships three impls:
//!   - [`facilitator::CdpFacilitator`] — real HTTP calls to the Coinbase
//!     CDP facilitator (`facilitator.coinbase.com`).
//!   - [`facilitator::HttpFacilitator`] — same wire format, configurable URL.
//!   - [`facilitator::MockFacilitator`] — in-process; verifies signatures
//!     against the payer's recovered public key, returns a deterministic
//!     synthetic tx hash. Used by tests and `x402t mock-facilitator`.
//! - [`X402Client`] — the high-level "POST a request, handle 402, sign,
//!   retry" loop. Built on `reqwest`.
//!
//! # Quickstart (offline, hermetic)
//!
//! ```
//! # tokio_test::block_on(async {
//! use x402_toolkit_client::{sign_authorization, MockFacilitator, LocalSigner, Facilitator, WalletSigner};
//! use x402_toolkit_types::{Network, PaymentSpec};
//!
//! let signer = LocalSigner::random();
//! let spec = PaymentSpec::usdc(Network::BaseSepolia, "1000", "0x9876543210987654321098765432109876543210");
//!
//! let payload = sign_authorization(&signer, &spec).await.unwrap();
//! let receipt = MockFacilitator::default().verify(&payload).await.unwrap();
//!
//! assert_eq!(receipt.payer.to_lowercase(), signer.address().to_lowercase());
//! # });
//! ```
//!
//! [`PaymentPayload`]: x402_toolkit_types::PaymentPayload

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod client;
pub mod error;
pub mod facilitator;
pub mod signer;

pub use client::X402Client;
pub use error::ClientError;
pub use facilitator::{
    CdpFacilitator, Facilitator, HttpFacilitator, MockFacilitator, PaymentRejection,
};
pub use signer::{LocalSigner, WalletSigner};

use rand::{thread_rng, RngCore as _};

use x402_toolkit_types::{
    eip3009::TransferWithAuthorization,
    payload::{Authorization, SignedAuthorization},
    PaymentPayload, PaymentSpec, Scheme,
};

/// Build a signed [`PaymentPayload`] for `spec`, paying from `signer`.
///
/// This composes a default `Authorization` (full `maxAmountRequired`,
/// `validAfter = 0`, `validBefore = now + spec.max_timeout_seconds`,
/// random nonce), produces the EIP-712 digest for it, signs the digest
/// via `signer.sign(&digest)`, and assembles the final
/// [`SignedAuthorization`].
///
/// For finer control (e.g. to set a non-default `validAfter` or a
/// caller-supplied nonce), use the [`PaymentBuilder`] API directly.
///
/// # Errors
///
/// Returns [`ClientError::InvalidSpec`] if the spec's `network` /
/// `asset` / `payTo` are malformed, [`ClientError::Signer`] if the signer
/// fails, or [`ClientError::Protocol`] for any other protocol-level
/// problem.
pub async fn sign_authorization<S: WalletSigner>(
    signer: &S,
    spec: &PaymentSpec,
) -> Result<PaymentPayload, ClientError> {
    PaymentBuilder::for_spec(spec).sign(signer).await
}

/// Composable builder for [`PaymentPayload`]s. Most callers want the
/// shorter [`sign_authorization`] free function.
pub struct PaymentBuilder<'a> {
    spec: &'a PaymentSpec,
    valid_after: u64,
    valid_before: Option<u64>,
    nonce: Option<[u8; 32]>,
    value: Option<String>,
}

impl<'a> PaymentBuilder<'a> {
    /// Begin a builder for `spec`. Defaults: full `maxAmountRequired`,
    /// `validAfter = 0`, `validBefore = now + spec.max_timeout_seconds`,
    /// random 32-byte nonce.
    pub fn for_spec(spec: &'a PaymentSpec) -> Self {
        Self {
            spec,
            valid_after: 0,
            valid_before: None,
            nonce: None,
            value: None,
        }
    }

    /// Override `validAfter`.
    pub fn valid_after(mut self, v: u64) -> Self {
        self.valid_after = v;
        self
    }

    /// Override `validBefore` (absolute unix-second timestamp).
    pub fn valid_before(mut self, v: u64) -> Self {
        self.valid_before = Some(v);
        self
    }

    /// Use a caller-supplied nonce (otherwise random).
    pub fn nonce(mut self, n: [u8; 32]) -> Self {
        self.nonce = Some(n);
        self
    }

    /// Pay less than `maxAmountRequired`. Most servers reject
    /// underpayments; only useful for protocol fuzzing or for schemes
    /// where the client picks the amount.
    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }

    /// Sign and assemble the final [`PaymentPayload`].
    pub async fn sign<S: WalletSigner>(self, signer: &S) -> Result<PaymentPayload, ClientError> {
        if !self.spec.scheme.is_exact() {
            return Err(ClientError::Protocol(format!(
                "unsupported scheme: {:?}; v0.1 only implements 'exact'",
                self.spec.scheme
            )));
        }

        let mut nonce_bytes = [0u8; 32];
        if let Some(n) = self.nonce {
            nonce_bytes = n;
        } else {
            thread_rng().fill_bytes(&mut nonce_bytes);
        }

        let valid_before = self.valid_before.unwrap_or_else(|| {
            chrono::Utc::now().timestamp() as u64 + u64::from(self.spec.max_timeout_seconds.max(1))
        });
        let value = self
            .value
            .unwrap_or_else(|| self.spec.max_amount_required.clone());

        let auth = Authorization {
            from: signer.address(),
            to: self.spec.pay_to.clone(),
            value,
            valid_after: self.valid_after.to_string(),
            valid_before: valid_before.to_string(),
            nonce: format!("0x{}", hex::encode(nonce_bytes)),
        };

        let twa = TransferWithAuthorization::from_wire(&auth)?;
        let (name, version) = domain_fields(self.spec);
        let digest = twa.eip712_hash(&name, &version, &self.spec.network, &self.spec.asset)?;
        let sig = signer.sign(&digest).await?;

        Ok(PaymentPayload {
            version: 2,
            scheme: Scheme::exact(),
            network: self.spec.network.clone(),
            payload: SignedAuthorization {
                signature: format!("0x{}", hex::encode(sig)),
                authorization: auth,
            },
        })
    }
}

/// Extract the EIP-712 `(name, version)` domain fields from a
/// [`PaymentSpec`]. Falls back to USDC defaults (`"USD Coin"` / `"2"`)
/// if `spec.extra` is missing.
pub(crate) fn domain_fields(spec: &PaymentSpec) -> (String, String) {
    let extra = spec.extra.as_ref();
    let name = extra
        .and_then(|e| e.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("USD Coin")
        .to_string();
    let version = extra
        .and_then(|e| e.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("2")
        .to_string();
    (name, version)
}
