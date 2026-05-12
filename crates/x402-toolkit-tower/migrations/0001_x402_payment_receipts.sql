-- x402-toolkit-tower: PgReceiptStore migration
--
-- Apply with sqlx-cli:
--   sqlx migrate add -r 0001_x402_payment_receipts
-- or import this file into an existing migration runner.

CREATE TABLE IF NOT EXISTS x402_payment_receipts (
    payment_id      TEXT        PRIMARY KEY,                       -- facilitator-issued
    wallet_address  VARCHAR(42) NOT NULL,
    nonce           BYTEA       NOT NULL,                          -- EIP-3009 nonce, 32 bytes
    tx_hash         VARCHAR(66),
    network         VARCHAR(32) NOT NULL,                          -- e.g. "eip155:8453"
    amount          NUMERIC(78, 0) NOT NULL,                       -- token base units
    currency        VARCHAR(64) NOT NULL,                          -- token contract address
    verified_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (wallet_address, nonce)
);

CREATE INDEX IF NOT EXISTS idx_x402_receipts_wallet
    ON x402_payment_receipts (wallet_address);
CREATE INDEX IF NOT EXISTS idx_x402_receipts_verified_at
    ON x402_payment_receipts (verified_at);
