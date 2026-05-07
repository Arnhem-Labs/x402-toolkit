//! Postgres-backed [`ReceiptStore`] implementation. Available behind the
//! `pg-store` feature.

use async_trait::async_trait;
use sqlx::{postgres::PgPool, Row};

use x402_toolkit_types::PaymentReceipt;

use crate::store::{ReceiptStore, StoreError};

/// Postgres-backed receipt store.
///
/// Apply the migration shipped at
/// `x402_toolkit_tower/migrations/0001_x402_payment_receipts.sql` before
/// using.
#[derive(Clone)]
pub struct PgReceiptStore {
    pool: PgPool,
}

impl PgReceiptStore {
    /// Wrap an existing `PgPool`.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReceiptStore for PgReceiptStore {
    async fn mark_seen(
        &self,
        receipt: &PaymentReceipt,
        nonce: &[u8; 32],
    ) -> Result<(), StoreError> {
        let payment_id = receipt
            .transaction
            .clone()
            .unwrap_or_else(|| format!("0x{}", hex::encode(nonce)));

        // Insert; on UNIQUE (wallet_address, nonce) violation, return Replayed.
        let result = sqlx::query(
            r"INSERT INTO x402_payment_receipts
              (payment_id, wallet_address, nonce, tx_hash, network, amount, currency, verified_at)
              VALUES ($1, $2, $3, $4, $5, $6::numeric, $7, $8)",
        )
        .bind(&payment_id)
        .bind(receipt.payer.to_lowercase())
        .bind(&nonce[..])
        .bind(receipt.transaction.as_deref())
        .bind(receipt.network.caip2())
        .bind("0") // amount placeholder — middleware doesn't carry amount yet
        .bind("") // currency placeholder
        .bind(receipt.verified_at.unwrap_or_else(chrono::Utc::now))
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Err(StoreError::Replayed),
            Err(e) => Err(StoreError::Backend(e.to_string())),
        }
    }

    async fn get(&self, payment_id: &str) -> Result<Option<PaymentReceipt>, StoreError> {
        let row = sqlx::query(
            r"SELECT wallet_address, tx_hash, network, verified_at
              FROM x402_payment_receipts
              WHERE payment_id = $1
              LIMIT 1",
        )
        .bind(payment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StoreError::Backend(e.to_string()))?;

        let Some(row) = row else { return Ok(None) };
        let payer: String = row
            .try_get("wallet_address")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let tx_hash: Option<String> = row
            .try_get("tx_hash")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let network_str: String = row
            .try_get("network")
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        let verified_at: chrono::DateTime<chrono::Utc> = row
            .try_get("verified_at")
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        let network = x402_toolkit_types::Network::parse_caip2(&network_str)
            .map_err(|e| StoreError::Backend(e.to_string()))?;

        Ok(Some(PaymentReceipt {
            success: true,
            transaction: tx_hash,
            network,
            payer,
            verified_at: Some(verified_at),
        }))
    }
}
