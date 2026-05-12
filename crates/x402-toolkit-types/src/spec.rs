//! `PaymentSpec` and `PaymentRequired` — the server-side challenge.
//!
//! When a server returns HTTP `402 Payment Required`, the body is a JSON
//! [`PaymentRequired`] wrapping one or more [`PaymentSpec`]s describing the
//! payments it will accept. Clients pick a spec, sign an EIP-3009 transfer
//! authorization for it, and retry the request with an `X-PAYMENT` header.

use serde::{Deserialize, Serialize};

use crate::Network;

/// Payment scheme. The x402 V2 spec defines `"exact"` (fixed-amount EIP-3009
/// `transferWithAuthorization`); future schemes will land here as variants.
///
/// The enum is `#[serde(untagged)]` so an unknown scheme name from a future
/// server doesn't fail deserialization — it lands in [`Scheme::Other`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scheme {
    /// A canonical, named scheme.
    Known(KnownScheme),
    /// An unknown scheme name. Forward-compatible escape hatch.
    Other(String),
}

/// Schemes this crate knows about by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnownScheme {
    /// Fixed-amount EIP-3009 `transferWithAuthorization`. The only scheme
    /// implemented end-to-end in v0.1.
    Exact,
}

impl Scheme {
    /// Convenience — the `"exact"` scheme.
    pub const fn exact() -> Self {
        Self::Known(KnownScheme::Exact)
    }

    /// `true` if this is the `"exact"` scheme.
    pub fn is_exact(&self) -> bool {
        matches!(self, Self::Known(KnownScheme::Exact))
    }
}

/// One acceptable payment terms, as advertised by the server in the body of
/// a `402 Payment Required` response.
///
/// Field names match the x402 V2 wire format. `maxAmountRequired` is a
/// **decimal string** of token base units (e.g. `"1000"` = 1000 micro-USDC =
/// $0.001 — USDC has 6 decimals).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentSpec {
    /// Payment scheme. Use [`Scheme::exact`] unless you have a reason not to.
    pub scheme: Scheme,
    /// Network the payment settles on, e.g. `Network::BaseMainnet`.
    pub network: Network,
    /// Amount in token base units, encoded as a decimal string. Strings are
    /// used because amounts can exceed `u64::MAX` for tokens with many
    /// decimals.
    pub max_amount_required: String,
    /// Resource URL the payment authorizes access to. Servers must echo the
    /// canonical URL of the gated endpoint here.
    pub resource: String,
    /// Optional human-readable description shown to the paying agent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub description: Option<String>,
    /// MIME type the gated resource will return on success. Defaults to
    /// `application/json` if omitted.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mime_type: Option<String>,
    /// EVM address that will receive the payment (the server's vault).
    pub pay_to: String,
    /// Maximum seconds the client has to complete the payment. Servers may
    /// reject signatures whose `validBefore` exceeds `now + this`.
    pub max_timeout_seconds: u32,
    /// EIP-20 token contract address.
    pub asset: String,
    /// Free-form scheme-specific data, e.g. for `"exact"`:
    /// `{ "name": "USD Coin", "version": "2" }` (the EIP-712 domain fields
    /// for the token's `transferWithAuthorization`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub extra: Option<serde_json::Value>,
}

impl PaymentSpec {
    /// Build a new `PaymentSpec` from the minimum required fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use x402_toolkit_types::{Network, PaymentSpec, Scheme};
    ///
    /// let spec = PaymentSpec::new(
    ///     Scheme::exact(),
    ///     Network::BaseSepolia,
    ///     "1000",
    ///     "https://api.example.com/v1/chat",
    ///     "0x9876543210987654321098765432109876543210",
    ///     "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
    ///     60,
    /// );
    /// assert!(spec.scheme.is_exact());
    /// ```
    pub fn new(
        scheme: Scheme,
        network: Network,
        max_amount_required: impl Into<String>,
        resource: impl Into<String>,
        pay_to: impl Into<String>,
        asset: impl Into<String>,
        max_timeout_seconds: u32,
    ) -> Self {
        Self {
            scheme,
            network,
            max_amount_required: max_amount_required.into(),
            resource: resource.into(),
            description: None,
            mime_type: None,
            pay_to: pay_to.into(),
            max_timeout_seconds,
            asset: asset.into(),
            extra: None,
        }
    }

    /// Build a USDC `PaymentSpec` on the given network.
    ///
    /// Defaults the asset to the network's canonical USDC contract, the
    /// scheme to `"exact"`, the resource to an empty string (set later via
    /// [`PaymentSpec::with_resource`]), the timeout to 60 seconds, and the
    /// EIP-712 domain `extra` field to `{"name":"USD Coin","version":"2"}`.
    ///
    /// # Panics
    ///
    /// Panics if the network does not have a known USDC address (i.e.
    /// [`Network::Custom`] with `usdc_address: None`).
    ///
    /// # Examples
    ///
    /// ```
    /// use x402_toolkit_types::{Network, PaymentSpec};
    ///
    /// let spec = PaymentSpec::usdc(
    ///     Network::BaseSepolia,
    ///     "1000",
    ///     "0x9876543210987654321098765432109876543210",
    /// )
    /// .with_resource("https://api.example.com/v1/chat");
    ///
    /// assert_eq!(spec.max_amount_required, "1000");
    /// assert_eq!(spec.asset, "0x036CbD53842c5426634e7929541eC2318f3dCF7e");
    /// ```
    pub fn usdc(
        network: Network,
        max_amount_required: impl Into<String>,
        pay_to: impl Into<String>,
    ) -> Self {
        let asset = network
            .usdc_address()
            .expect("Network::Custom without a usdc_address; pass it explicitly via new()")
            .to_string();
        Self {
            scheme: Scheme::exact(),
            network,
            max_amount_required: max_amount_required.into(),
            resource: String::new(),
            description: None,
            mime_type: None,
            pay_to: pay_to.into(),
            max_timeout_seconds: 60,
            asset,
            extra: Some(serde_json::json!({ "name": "USD Coin", "version": "2" })),
        }
    }

    /// Set the resource URL.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = resource.into();
        self
    }

    /// Set a human-readable description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the maximum payment timeout (seconds). Default is 60.
    pub fn with_max_timeout_seconds(mut self, secs: u32) -> Self {
        self.max_timeout_seconds = secs;
        self
    }

    /// Override the canonical EIP-712 domain `extra` field. Most callers
    /// won't need this — the [`PaymentSpec::usdc`] constructor sets the
    /// USDC defaults (`{"name":"USD Coin","version":"2"}`).
    pub fn with_extra(mut self, extra: serde_json::Value) -> Self {
        self.extra = Some(extra);
        self
    }
}

/// Body of a `402 Payment Required` response. Carries one or more
/// [`PaymentSpec`]s (the server may offer multiple options — different
/// networks, different prices, different schemes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentRequired {
    /// Protocol version. Currently always `"2"` for x402 V2.
    pub version: String,
    /// One or more accepted payment specs.
    pub accepts: Vec<PaymentSpec>,
}

impl PaymentRequired {
    /// Build a `PaymentRequired` with a single accepted spec.
    pub fn single(spec: PaymentSpec) -> Self {
        Self {
            version: crate::X402_VERSION.to_string(),
            accepts: vec![spec],
        }
    }

    /// Build a `PaymentRequired` with multiple accepted specs.
    pub fn many(specs: impl IntoIterator<Item = PaymentSpec>) -> Self {
        Self {
            version: crate::X402_VERSION.to_string(),
            accepts: specs.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_spec() -> PaymentSpec {
        PaymentSpec::usdc(
            Network::BaseSepolia,
            "1000",
            "0x9876543210987654321098765432109876543210",
        )
        .with_resource("https://api.example.com/v1/chat")
        .with_description("$0.001 USDC per call")
    }

    #[test]
    fn serializes_with_camel_case_and_v2_version() {
        let pr = PaymentRequired::single(sample_spec());
        let v: serde_json::Value = serde_json::to_value(&pr).unwrap();
        assert_eq!(v["version"], "2");
        let s = &v["accepts"][0];
        assert_eq!(s["scheme"], "exact");
        assert_eq!(s["network"], "eip155:84532");
        assert_eq!(s["maxAmountRequired"], "1000");
        assert_eq!(s["resource"], "https://api.example.com/v1/chat");
        assert_eq!(s["payTo"], "0x9876543210987654321098765432109876543210");
        assert_eq!(s["maxTimeoutSeconds"], 60);
        assert_eq!(s["asset"], "0x036CbD53842c5426634e7929541eC2318f3dCF7e");
    }

    #[test]
    fn roundtrip_single() {
        let pr = PaymentRequired::single(sample_spec());
        let json = serde_json::to_string(&pr).unwrap();
        let back: PaymentRequired = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pr);
    }

    #[test]
    fn forward_compat_unknown_scheme() {
        let json = r#"{
            "scheme": "future-scheme",
            "network": "eip155:8453",
            "maxAmountRequired": "1000",
            "resource": "https://example.com",
            "payTo": "0x0000000000000000000000000000000000000001",
            "maxTimeoutSeconds": 60,
            "asset": "0x0000000000000000000000000000000000000002"
        }"#;
        let s: PaymentSpec = serde_json::from_str(json).unwrap();
        assert!(matches!(s.scheme, Scheme::Other(ref t) if t == "future-scheme"));
    }

    #[test]
    fn omits_none_fields() {
        let s = PaymentSpec::new(
            Scheme::exact(),
            Network::BaseMainnet,
            "1000",
            "https://example.com",
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            60,
        );
        let v = serde_json::to_value(&s).unwrap();
        assert!(v.get("description").is_none());
        assert!(v.get("mimeType").is_none());
        assert!(v.get("extra").is_none());
    }
}
