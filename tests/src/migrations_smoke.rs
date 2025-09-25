use std::path::Path;
use tokio::process::Command;
use tokio::fs;
use sqlx::{SqlitePool, Row};

#[tokio::test]
async fn migrations_apply_on_fresh_db() {
    // Use a temp directory under ./data/test_migrations
    let test_dir = Path::new("./data/test_migrations");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).await.ok();
    }
    fs::create_dir_all(test_dir).await.expect("create test dir");

    // Point DB to a fresh file
    let db_url = format!("sqlite://{}/smoke.db", test_dir.to_string_lossy());

    // Run the server in migrate-only mode with env var for DB
    let status = Command::new("cargo")
        .arg("run")
        .arg("-p").arg("blockchain-server")
        .arg("--")
        .arg("--migrate")
        .env("BLOCKCHAIN_DATABASE_URL", &db_url)
        .status()
        .await
        .expect("spawn migrate");
    assert!(status.success(), "migrate command failed");

    // Connect to the DB and verify migrations
    let pool = SqlitePool::connect(&db_url).await.expect("connect db");
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM schema_migrations WHERE filename IN ('0000_init.sql','0001_identity_commitments.sql')"
    )
    .fetch_one(&pool).await.expect("count migrations");
    assert_eq!(count, 2, "expected two migrations applied");

    // Ensure identity_commitments table exists by querying sqlite_master
    let row = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='identity_commitments'"
    )
    .fetch_one(&pool).await.expect("find identity_commitments");
    let name: String = row.get("name");
    assert_eq!(name, "identity_commitments");
}
