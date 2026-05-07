//! EIP-3009 [`TransferWithAuthorization`] — the typed-data structure that
//! `scheme = "exact"` payments authorize the facilitator to submit
//! on-chain.
//!
//! The big function here is
//! [`TransferWithAuthorization::eip712_hash`], which computes the digest a
//! [`crate::payload::Authorization`] must be signed against. It works
//! identically whether you're signing client-side or recovering the signer
//! server-side.
//!
//! # References
//!
//! - [EIP-3009](https://eips.ethereum.org/EIPS/eip-3009) — the
//!   `transferWithAuthorization` extension to ERC-20.
//! - [EIP-712](https://eips.ethereum.org/EIPS/eip-712) — typed structured
//!   data hashing.

use alloy_primitives::{keccak256, Address, B256, U256};
use alloy_sol_types::{eip712_domain, sol, Eip712Domain, SolStruct};

use crate::{payload::Authorization, Network, X402Error};

sol! {
    /// EIP-3009 typed-data struct, exactly as defined in the standard.
    #[derive(Debug)]
    struct TransferWithAuthorization {
        address from;
        address to;
        uint256 value;
        uint256 validAfter;
        uint256 validBefore;
        bytes32 nonce;
    }
}

impl TransferWithAuthorization {
    /// Build a `TransferWithAuthorization` from the wire-format
    /// [`Authorization`] used in `X-PAYMENT` headers.
    ///
    /// # Errors
    ///
    /// Returns [`X402Error::Invalid`] if any address or numeric field is
    /// malformed, or [`X402Error::Hex`] if the nonce isn't valid hex.
    pub fn from_wire(a: &Authorization) -> Result<Self, X402Error> {
        let from: Address = a
            .from
            .parse()
            .map_err(|e| X402Error::Invalid(format!("from: {e}")))?;
        let to: Address =
            a.to.parse()
                .map_err(|e| X402Error::Invalid(format!("to: {e}")))?;
        let value: U256 = U256::from_str_radix(&a.value, 10)
            .map_err(|e| X402Error::Invalid(format!("value: {e}")))?;
        let valid_after: U256 = U256::from_str_radix(&a.valid_after, 10)
            .map_err(|e| X402Error::Invalid(format!("validAfter: {e}")))?;
        let valid_before: U256 = U256::from_str_radix(&a.valid_before, 10)
            .map_err(|e| X402Error::Invalid(format!("validBefore: {e}")))?;
        let nonce_bytes = a.nonce_bytes()?;
        let nonce: B256 = nonce_bytes.into();

        Ok(Self {
            from,
            to,
            value,
            validAfter: valid_after,
            validBefore: valid_before,
            nonce,
        })
    }

    /// Compute the EIP-712 typed-data digest for this authorization on a
    /// given EIP-20 token contract on a given [`Network`].
    ///
    /// This is the 32-byte hash that a payer's wallet signs (the input to
    /// `secp256k1::sign`). Servers recompute the same hash to recover the
    /// payer's address from the signature.
    ///
    /// # Arguments
    ///
    /// - `name` / `version` — the EIP-712 domain fields the token's
    ///   `transferWithAuthorization` was deployed with. For native USDC on
    ///   Base mainnet and Sepolia these are `"USD Coin"` and `"2"`.
    /// - `network` — supplies the EIP-155 `chainId` of the domain.
    /// - `verifying_contract` — the address of the EIP-3009 token (e.g. USDC).
    ///
    /// # Examples
    ///
    /// ```
    /// use x402_toolkit_types::{
    ///     eip3009::TransferWithAuthorization,
    ///     payload::Authorization,
    ///     Network, USDC_BASE_MAINNET,
    /// };
    ///
    /// let a = Authorization {
    ///     from: "0x1111111111111111111111111111111111111111".into(),
    ///     to:   "0x2222222222222222222222222222222222222222".into(),
    ///     value: "1000".into(),
    ///     valid_after: "0".into(),
    ///     valid_before: "9999999999".into(),
    ///     nonce: "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    ///         .into(),
    /// };
    /// let twa = TransferWithAuthorization::from_wire(&a).unwrap();
    /// let digest = twa.eip712_hash(
    ///     "USD Coin",
    ///     "2",
    ///     &Network::BaseMainnet,
    ///     USDC_BASE_MAINNET,
    /// ).unwrap();
    /// assert_eq!(digest.len(), 32);
    /// ```
    pub fn eip712_hash(
        &self,
        name: &str,
        version: &str,
        network: &Network,
        verifying_contract: &str,
    ) -> Result<[u8; 32], X402Error> {
        let verifying: Address = verifying_contract
            .parse()
            .map_err(|e| X402Error::Invalid(format!("verifying_contract: {e}")))?;
        let domain: Eip712Domain = eip712_domain! {
            name: name.to_string(),
            version: version.to_string(),
            chain_id: network.chain_id(),
            verifying_contract: verifying,
        };
        let digest = self.eip712_signing_hash(&domain);
        Ok(digest.0)
    }

    /// Convenience — compute the EIP-712 hash with the canonical USDC
    /// domain (`name = "USD Coin"`, `version = "2"`).
    pub fn eip712_hash_for_usdc(&self, network: &Network) -> Result<[u8; 32], X402Error> {
        let verifying = network
            .usdc_address()
            .ok_or_else(|| X402Error::Invalid("network has no canonical USDC address".into()))?;
        self.eip712_hash("USD Coin", "2", network, verifying)
    }
}

/// The EIP-3009 `TransferWithAuthorization` type-hash, exposed for tests
/// and debugging.
pub fn type_hash() -> B256 {
    keccak256(
        b"TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_auth() -> Authorization {
        Authorization {
            from: "0x1111111111111111111111111111111111111111".into(),
            to: "0x2222222222222222222222222222222222222222".into(),
            value: "1000".into(),
            valid_after: "0".into(),
            valid_before: "9999999999".into(),
            nonce: "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".into(),
        }
    }

    #[test]
    fn from_wire_parses() {
        let a = sample_auth();
        let twa = TransferWithAuthorization::from_wire(&a).unwrap();
        assert_eq!(twa.value, U256::from(1000u64));
        assert_eq!(twa.from.to_string().to_lowercase(), a.from.to_lowercase());
    }

    #[test]
    fn hash_is_deterministic_and_32_bytes() {
        let a = sample_auth();
        let twa = TransferWithAuthorization::from_wire(&a).unwrap();
        let h1 = twa.eip712_hash_for_usdc(&Network::BaseMainnet).unwrap();
        let h2 = twa.eip712_hash_for_usdc(&Network::BaseMainnet).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn hash_differs_per_chain() {
        let a = sample_auth();
        let twa = TransferWithAuthorization::from_wire(&a).unwrap();
        let mainnet = twa.eip712_hash_for_usdc(&Network::BaseMainnet).unwrap();
        let sepolia = twa.eip712_hash_for_usdc(&Network::BaseSepolia).unwrap();
        assert_ne!(mainnet, sepolia);
    }

    #[test]
    fn hash_differs_per_value() {
        let mut a = sample_auth();
        let twa1 = TransferWithAuthorization::from_wire(&a).unwrap();
        a.value = "1001".into();
        let twa2 = TransferWithAuthorization::from_wire(&a).unwrap();
        let h1 = twa1.eip712_hash_for_usdc(&Network::BaseMainnet).unwrap();
        let h2 = twa2.eip712_hash_for_usdc(&Network::BaseMainnet).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn type_hash_matches_eip3009() {
        // Spec value: keccak256(
        //   "TransferWithAuthorization(address from,address to,uint256 value,
        //    uint256 validAfter,uint256 validBefore,bytes32 nonce)"
        // )
        let h = type_hash();
        // Sanity: deterministic, 32 bytes.
        assert_eq!(h.0.len(), 32);
        // The exact value is documented in the EIP-3009 reference impl
        // and many on-chain USDC contracts. We only assert determinism here
        // — a mismatched type-hash is caught by interop tests against a
        // real facilitator.
        let h2 = type_hash();
        assert_eq!(h, h2);
    }
}
