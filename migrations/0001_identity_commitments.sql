-- Migration 0001: Create identity_commitments table and indexes
-- Date: 2025-09-25
-- Description: Introduces table to store hashed identity credential commitments (Phase 2)

CREATE TABLE IF NOT EXISTS identity_commitments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    public_key TEXT NOT NULL,
    did TEXT NOT NULL,
    commitment_hash TEXT NOT NULL UNIQUE,
    issuer_did TEXT NOT NULL,
    issued_at TEXT NOT NULL,
    expires_at TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_identity_did ON identity_commitments (did);
CREATE INDEX IF NOT EXISTS idx_identity_issuer ON identity_commitments (issuer_did);
