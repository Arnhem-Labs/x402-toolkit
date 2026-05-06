//! `PaymentPayload` — the decoded contents of an `X-PAYMENT` header.
//!
//! When a client retries a `402` request, it sets the `X-PAYMENT` header
//! to a base64-encoded JSON [`PaymentPayload`]. The payload bundles a
//! signed EIP-3009 [`Authorization`] with the `signature` produced by the
//! payer's wallet.

use serde::{Deserialize, Serialize};

use crate::{Network, Scheme};

/// Decoded body of the `X-PAYMENT` header.
///
/// # Wire format
///
/// ```json
/// {
///   "x402Version": 2,
///   "scheme": "exact",
///   "network": "eip155:8453",
///   "payload": {
///     "signature": "0x...",
///     "authorization": {
///       "from": "0xPAYER",
///       "to": "0xVAULT",
///       "value": "1000",
///       "validAfter": "0",
///       "validBefore": "1744000000",
///       "nonce": "0x...random_bytes32"
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentPayload {
    /// Protocol version, encoded as an integer (`2`).
    #[serde(rename = "x402Version")]
    pub version: u32,
    /// Scheme — must match one of the [`PaymentSpec.scheme`](crate::PaymentSpec)
    /// values from the original challenge.
    pub scheme: Scheme,
    /// Network the payment settles on.
    pub network: Network,
    /// The signed authorization itself.
    pub payload: SignedAuthorization,
}

/// A signed EIP-3009 [`Authorization`] paired with its `signature`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedAuthorization {
    /// `0x`-prefixed 65-byte secp256k1 signature in `r || s || v` form.
    pub signature: String,
    /// The transfer authorization that was signed.
    pub authorization: Authorization,
}

/// EIP-3009 `transferWithAuthorization` parameters as exchanged on the
/// wire.
///
/// `value`, `validAfter`, and `validBefore` are decimal strings to permit
/// values larger than `u64::MAX` and to match the wire format of the
/// reference TS/Python/Go SDKs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    /// EVM address paying.
    pub from: String,
    /// EVM address receiving.
    pub to: String,
    /// Amount in token base units, decimal string.
    pub value: String,
    /// Earliest unix-second timestamp at which the transfer is valid.
    pub valid_after: String,
    /// Latest unix-second timestamp at which the transfer is valid.
    pub valid_before: String,
    /// Random 32-byte nonce, `0x`-prefixed hex.
    pub nonce: String,
}

impl Authorization {
    /// Decode `nonce` into a 32-byte array.
    ///
    /// # Errors
    ///
    /// Returns [`crate::X402Error::Invalid`] if the nonce is the wrong length
    /// or [`crate::X402Error::Hex`] if it isn't valid hex.
    pub fn nonce_bytes(&self) -> Result<[u8; 32], crate::X402Error> {
        let trimmed = self.nonce.strip_prefix("0x").unwrap_or(&self.nonce);
        let v = hex::decode(trimmed)?;
        v.as_slice()
            .try_into()
            .map_err(|_| crate::X402Error::Invalid(format!("nonce must be 32 bytes, got {}", v.len())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_payload() {
        let p = PaymentPayload {
            version: 2,
            scheme: Scheme::exact(),
            network: Network::BaseMainnet,
            payload: SignedAuthorization {
                signature: "0xabcd".into(),
                authorization: Authorization {
                    from: "0x1111111111111111111111111111111111111111".into(),
                    to: "0x2222222222222222222222222222222222222222".into(),
                    value: "1000".into(),
                    valid_after: "0".into(),
                    valid_before: "1744000000".into(),
                    nonce: "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
                        .into(),
                },
            },
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["x402Version"], 2);
        assert_eq!(json["scheme"], "exact");
        assert_eq!(json["payload"]["authorization"]["validBefore"], "1744000000");

        let back: PaymentPayload = serde_json::from_value(json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn nonce_bytes_decodes_hex() {
        let a = Authorization {
            from: String::new(),
            to: String::new(),
            value: "0".into(),
            valid_after: "0".into(),
            valid_before: "0".into(),
            nonce: "0x000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
                .into(),
        };
        let bytes = a.nonce_bytes().unwrap();
        assert_eq!(bytes[0], 0x00);
        assert_eq!(bytes[31], 0x1f);
    }

    #[test]
    fn nonce_bytes_rejects_wrong_length() {
        let a = Authorization {
            from: String::new(),
            to: String::new(),
            value: "0".into(),
            valid_after: "0".into(),
            valid_before: "0".into(),
            nonce: "0x00".into(),
        };
        assert!(a.nonce_bytes().is_err());
    }
}
