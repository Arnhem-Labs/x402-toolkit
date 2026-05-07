//! [`WalletSigner`] trait — sign a 32-byte digest, return a 65-byte
//! secp256k1 signature.
//!
//! Why not depend directly on `alloy_signer::Signer`? Three reasons:
//!
//! 1. Alloy's `Signer` is generic over hash type and has methods we don't
//!    need (`sign_message_*`, `sign_typed_data_*`, `sign_hash`). Our
//!    surface is one method.
//! 2. Async wallets (KMS, Ledger, browser-injected) are awkward to wedge
//!    behind alloy's signature-async-on-blocking machinery; an explicit
//!    `async fn sign(&self, &[u8; 32])` is cleaner.
//! 3. Decoupling lets non-EVM signers (placeholder for non-EIP-3009
//!    schemes in v0.2+) implement the trait without an alloy dep.
//!
//! `LocalSigner` adapts an alloy local signer for the happy path.

use async_trait::async_trait;

use crate::ClientError;

/// Sign a 32-byte digest, return a 65-byte secp256k1 signature in
/// `r || s || v` form (the layout expected by EIP-712 / EIP-3009).
///
/// Implementations are expected to be cheap to clone (typically
/// `Arc<...>`); the `&self` signature lets them be shared across async
/// tasks.
#[async_trait]
pub trait WalletSigner: Send + Sync + 'static {
    /// EIP-55-checksummed (or simple lower-case hex) address of the
    /// wallet.
    fn address(&self) -> String;

    /// Sign a 32-byte EIP-712 digest, return the 65-byte signature.
    async fn sign(&self, digest: &[u8; 32]) -> Result<[u8; 65], ClientError>;
}

/// In-process signer backed by a 32-byte private key. Convenient for
/// tests, the `x402t sign` CLI subcommand, and CI smoke tests against
/// Base Sepolia.
///
/// Production deployments should implement [`WalletSigner`] for an HSM,
/// AWS KMS, GCP KMS, Ledger, or another secure key custody system rather
/// than holding raw keys in process.
///
/// # Examples
///
/// ```
/// # tokio_test::block_on(async {
/// use x402_toolkit_client::{LocalSigner, WalletSigner};
///
/// let signer = LocalSigner::random();
/// let digest = [0u8; 32];
/// let sig = signer.sign(&digest).await.unwrap();
/// assert_eq!(sig.len(), 65);
/// assert!(signer.address().starts_with("0x"));
/// # })
/// ```
#[derive(Clone)]
pub struct LocalSigner {
    inner: alloy_signer_local::PrivateKeySigner,
}

impl LocalSigner {
    /// Generate a fresh random key.
    pub fn random() -> Self {
        Self {
            inner: alloy_signer_local::PrivateKeySigner::random(),
        }
    }

    /// Build from a 32-byte hex private key (with or without `0x`
    /// prefix).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Signer`] if the input isn't 32 bytes of
    /// valid hex or if the key is invalid for secp256k1.
    pub fn from_hex(s: &str) -> Result<Self, ClientError> {
        let trimmed = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(trimmed).map_err(|e| ClientError::Signer(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(ClientError::Signer(format!(
                "private key must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        let inner = alloy_signer_local::PrivateKeySigner::from_slice(&bytes)
            .map_err(|e| ClientError::Signer(e.to_string()))?;
        Ok(Self { inner })
    }

    /// 32-byte private key as a `0x`-prefixed lower-case hex string.
    /// Use sparingly — exporting keys defeats the purpose of holding
    /// them.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(self.inner.to_bytes()))
    }
}

#[async_trait]
impl WalletSigner for LocalSigner {
    fn address(&self) -> String {
        format!("{}", self.inner.address())
    }

    async fn sign(&self, digest: &[u8; 32]) -> Result<[u8; 65], ClientError> {
        use alloy_primitives::B256;
        use alloy_signer::SignerSync;

        let hash = B256::from_slice(digest);
        let sig = self
            .inner
            .sign_hash_sync(&hash)
            .map_err(|e| ClientError::Signer(e.to_string()))?;
        // alloy::Signature::as_bytes() is r || s || v with v in {27,28}
        // since 0.5; that matches the EIP-3009 expected shape.
        Ok(sig.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn random_signer_signs() {
        let s = LocalSigner::random();
        let digest = [42u8; 32];
        let sig = s.sign(&digest).await.unwrap();
        assert_eq!(sig.len(), 65);
        assert!(s.address().starts_with("0x"));
    }

    #[tokio::test]
    async fn from_hex_roundtrip() {
        let original = LocalSigner::random();
        let hex = original.to_hex();
        let restored = LocalSigner::from_hex(&hex).unwrap();
        assert_eq!(original.address(), restored.address());
    }

    #[tokio::test]
    async fn from_hex_rejects_garbage() {
        assert!(LocalSigner::from_hex("0xZZ").is_err());
        assert!(LocalSigner::from_hex("abcd").is_err()); // wrong length
    }

    #[tokio::test]
    async fn signature_is_deterministic_for_same_key_and_digest() {
        let s = LocalSigner::random();
        let d = [1u8; 32];
        let a = s.sign(&d).await.unwrap();
        let b = s.sign(&d).await.unwrap();
        assert_eq!(a, b);
    }
}
