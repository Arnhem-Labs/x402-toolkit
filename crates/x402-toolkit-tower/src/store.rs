//! [`ReceiptStore`] trait for nonce-replay protection + the always-on
//! [`InMemoryStore`] impl.
//!
//! See `pg::PgReceiptStore` (feature `pg-store`) for a Postgres impl.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use thiserror::Error;

use x402_toolkit_types::PaymentReceipt;

/// Errors raised by [`ReceiptStore`] impls.
#[derive(Debug, Error)]
pub enum StoreError {
    /// The (wallet, nonce) pair has been seen before within the TTL.
    #[error("nonce replayed")]
    Replayed,
    /// Backend (DB, Redis, …) error.
    #[error("backend: {0}")]
    Backend(String),
}

/// Persist verified [`PaymentReceipt`]s long enough to detect nonce
/// replays.
///
/// Two operations:
///
/// - [`mark_seen`](ReceiptStore::mark_seen): atomically check
///   `(wallet, nonce)` and, if unseen, persist the receipt. Returns
///   [`StoreError::Replayed`] on duplicate.
/// - [`get`](ReceiptStore::get): retrieve by `payment_id`, optional —
///   used by audit / debug tooling. Implementations may return
///   `Ok(None)` if they don't want to support lookups.
#[async_trait]
pub trait ReceiptStore: Send + Sync + 'static {
    /// Atomic insert. Returns `Err(StoreError::Replayed)` if the
    /// `(wallet, nonce)` was seen before its TTL expired.
    async fn mark_seen(&self, receipt: &PaymentReceipt, nonce: &[u8; 32]) -> Result<(), StoreError>;

    /// Optional point-lookup by `payment_id`. Implementations that
    /// don't store full receipts can return `Ok(None)`.
    async fn get(&self, payment_id: &str) -> Result<Option<PaymentReceipt>, StoreError>;
}

/// Thread-safe in-memory `ReceiptStore` with a TTL.
///
/// Suitable for single-instance deployments and tests. For multi-region
/// or multi-instance Lambda fleets, use a shared store (Postgres via
/// `pg-store` feature, Redis, DynamoDB, …).
pub struct InMemoryStore {
    inner: Mutex<HashMap<Key, Entry>>,
    ttl: Duration,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct Key {
    wallet_lower: String,
    nonce: [u8; 32],
}

struct Entry {
    receipt: PaymentReceipt,
    seen_at: Instant,
}

impl InMemoryStore {
    /// Build a store with a 5-minute TTL.
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(300))
    }

    /// Build a store with a caller-specified TTL. Entries older than
    /// `ttl` are pruned at insert time.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    fn prune(&self, map: &mut HashMap<Key, Entry>) {
        let cutoff = Instant::now().checked_sub(self.ttl);
        if let Some(c) = cutoff {
            map.retain(|_, e| e.seen_at > c);
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReceiptStore for InMemoryStore {
    async fn mark_seen(&self, receipt: &PaymentReceipt, nonce: &[u8; 32]) -> Result<(), StoreError> {
        let mut map = self.inner.lock().map_err(|_| StoreError::Backend("poisoned".into()))?;
        self.prune(&mut map);
        let key = Key {
            wallet_lower: receipt.payer.to_lowercase(),
            nonce: *nonce,
        };
        if map.contains_key(&key) {
            return Err(StoreError::Replayed);
        }
        map.insert(
            key,
            Entry {
                receipt: receipt.clone(),
                seen_at: Instant::now(),
            },
        );
        Ok(())
    }

    async fn get(&self, payment_id: &str) -> Result<Option<PaymentReceipt>, StoreError> {
        let map = self.inner.lock().map_err(|_| StoreError::Backend("poisoned".into()))?;
        Ok(map
            .values()
            .find(|e| {
                // payment_id is a tx_hash here — pragmatic: tx_hash is
                // the natural primary key and we don't store any other
                // id.
                e.receipt.transaction.as_deref() == Some(payment_id)
            })
            .map(|e| e.receipt.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x402_toolkit_types::Network;

    fn receipt(payer: &str, tx: &str) -> PaymentReceipt {
        PaymentReceipt::ok(Network::BaseSepolia, payer).with_transaction(tx)
    }

    #[tokio::test]
    async fn mark_seen_then_get_returns_some() {
        let s = InMemoryStore::new();
        let r = receipt("0xPAYER", "0xtx1");
        let nonce = [1u8; 32];
        s.mark_seen(&r, &nonce).await.unwrap();
        let back = s.get("0xtx1").await.unwrap().unwrap();
        assert_eq!(back.payer, "0xPAYER");
    }

    #[tokio::test]
    async fn replay_is_rejected() {
        let s = InMemoryStore::new();
        let r = receipt("0xPAYER", "0xtx1");
        let nonce = [2u8; 32];
        s.mark_seen(&r, &nonce).await.unwrap();
        match s.mark_seen(&r, &nonce).await {
            Err(StoreError::Replayed) => {}
            other => panic!("expected Replayed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn different_nonce_same_wallet_is_fine() {
        let s = InMemoryStore::new();
        let r = receipt("0xPAYER", "0xtx1");
        s.mark_seen(&r, &[1u8; 32]).await.unwrap();
        s.mark_seen(&r, &[2u8; 32]).await.unwrap();
    }

    #[tokio::test]
    async fn ttl_prunes() {
        let s = InMemoryStore::with_ttl(Duration::from_millis(1));
        let r = receipt("0xPAYER", "0xtx1");
        s.mark_seen(&r, &[1u8; 32]).await.unwrap();
        std::thread::sleep(Duration::from_millis(20));
        // After TTL, the same nonce can be reused (because the prior
        // entry is pruned on the next insert).
        s.mark_seen(&r, &[1u8; 32]).await.unwrap();
    }
}
