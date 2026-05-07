//! EVM network identifiers as used by the x402 protocol.
//!
//! The protocol uses CAIP-2-style identifiers: `eip155:<chain_id>`.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Identifies the EVM network a payment settles on.
///
/// The variants are an *open* enum: well-known networks have named variants,
/// and [`Network::Custom`] accepts any chain ID. New canonical variants can
/// be added in minor releases without breaking downstream code that was
/// already using `Custom { chain_id, .. }` for the same chain.
///
/// # Examples
///
/// ```
/// use x402_toolkit_types::Network;
///
/// assert_eq!(Network::BaseMainnet.caip2(), "eip155:8453");
/// assert_eq!(Network::BaseSepolia.chain_id(), 84_532);
///
/// // Optimism mainnet via the escape hatch:
/// let op = Network::Custom { chain_id: 10, name: "optimism".into(), usdc_address: None };
/// assert_eq!(op.caip2(), "eip155:10");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    /// Base mainnet (`eip155:8453`).
    BaseMainnet,
    /// Base Sepolia testnet (`eip155:84532`).
    BaseSepolia,
    /// Any EVM chain not represented by a named variant.
    Custom {
        /// EIP-155 chain id (e.g. `10` for Optimism, `42161` for Arbitrum One).
        chain_id: u64,
        /// Human-friendly name for logs and error messages.
        name: Cow<'static, str>,
        /// Canonical USDC contract on this chain, if known. Optional — most
        /// of the protocol works fine without it; consumers that want to
        /// default `PaymentSpec.asset` need it.
        usdc_address: Option<String>,
    },
}

impl Network {
    /// EIP-155 chain id.
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::BaseMainnet => 8_453,
            Self::BaseSepolia => 84_532,
            Self::Custom { chain_id, .. } => *chain_id,
        }
    }

    /// CAIP-2 identifier as a `String`. The on-the-wire form used in
    /// [`PaymentSpec.network`](crate::PaymentSpec::network).
    pub fn caip2(&self) -> String {
        format!("eip155:{}", self.chain_id())
    }

    /// Canonical USDC contract on this network, when known.
    pub fn usdc_address(&self) -> Option<&str> {
        match self {
            Self::BaseMainnet => Some(crate::USDC_BASE_MAINNET),
            Self::BaseSepolia => Some(crate::USDC_BASE_SEPOLIA),
            Self::Custom { usdc_address, .. } => usdc_address.as_deref(),
        }
    }

    /// `true` if this network is a testnet — useful for guarding
    /// production-only code paths.
    pub fn is_testnet(&self) -> bool {
        matches!(self, Self::BaseSepolia)
    }

    /// Parse a CAIP-2 identifier (`eip155:<chain_id>`) into a known network
    /// where possible, falling back to [`Network::Custom`] otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`crate::X402Error::Invalid`] if the input is not in
    /// `eip155:<u64>` form.
    pub fn parse_caip2(s: &str) -> Result<Self, crate::X402Error> {
        let rest = s.strip_prefix("eip155:").ok_or_else(|| {
            crate::X402Error::Invalid(format!(
                "network identifier must start with 'eip155:', got {s:?}"
            ))
        })?;
        let chain_id: u64 = rest
            .parse()
            .map_err(|_| crate::X402Error::Invalid(format!("invalid chain id in {s:?}")))?;
        Ok(match chain_id {
            8_453 => Self::BaseMainnet,
            84_532 => Self::BaseSepolia,
            other => Self::Custom {
                chain_id: other,
                name: format!("eip155-{other}").into(),
                usdc_address: None,
            },
        })
    }
}

impl Serialize for Network {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.caip2())
    }
}

impl<'de> Deserialize<'de> for Network {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse_caip2(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caip2_roundtrip_named_variants() {
        for net in [Network::BaseMainnet, Network::BaseSepolia] {
            let s = net.caip2();
            let parsed = Network::parse_caip2(&s).unwrap();
            assert_eq!(parsed, net);
        }
    }

    #[test]
    fn caip2_roundtrip_custom_via_serde() {
        let s = "eip155:10";
        let parsed = Network::parse_caip2(s).unwrap();
        assert_eq!(parsed.chain_id(), 10);
        assert_eq!(parsed.caip2(), s);
    }

    #[test]
    fn parse_rejects_non_eip155() {
        assert!(Network::parse_caip2("solana:mainnet").is_err());
        assert!(Network::parse_caip2("eip155:not-a-number").is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let n = Network::BaseSepolia;
        let json = serde_json::to_string(&n).unwrap();
        assert_eq!(json, "\"eip155:84532\"");
        let back: Network = serde_json::from_str(&json).unwrap();
        assert_eq!(back, n);
    }
}
