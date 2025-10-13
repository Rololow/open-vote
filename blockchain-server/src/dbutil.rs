use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::str::FromStr;

/// Public helper to ensure minimal DB schema for tests and integration
pub async fn ensure_minimal_schema(db_url: &str) {
    let opts = SqliteConnectOptions::from_str(db_url).unwrap().create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await.unwrap();
    // blocks table (subset sufficient for load_blockchain query)
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS blocks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_number INTEGER NOT NULL UNIQUE,
            hash TEXT NOT NULL UNIQUE,
            previous_hash TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            block_data TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // identity_commitments table needed by identity validation
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS identity_commitments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            public_key TEXT NOT NULL,
            did TEXT NOT NULL,
            commitment_hash TEXT NOT NULL UNIQUE,
            issuer_did TEXT NOT NULL,
            issued_at TEXT NOT NULL,
            expires_at TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(&pool)
    .await
    .unwrap();
}
