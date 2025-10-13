-- Phase 3: store nullifiers to enforce one-person-one-action per scope
CREATE TABLE IF NOT EXISTS nullifiers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scope TEXT NOT NULL,
    nullifier_hex TEXT NOT NULL,
    tx_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(scope, nullifier_hex)
);
