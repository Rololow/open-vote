use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use tracing::{info, warn};

/// Revocation status for credentials and identities (Phase 6.3)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum RevocationReason {
    /// Key compromise detected
    KeyCompromise,
    /// Credential expired naturally
    Expired,
    /// Administrative revocation
    Administrative,
    /// Identity verification failed on re-check
    IdentityInvalid,
    /// User requested revocation
    UserRequested,
}

impl RevocationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KeyCompromise => "key_compromise",
            Self::Expired => "expired",
            Self::Administrative => "administrative",
            Self::IdentityInvalid => "identity_invalid",
            Self::UserRequested => "user_requested",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "key_compromise" => Some(Self::KeyCompromise),
            "expired" => Some(Self::Expired),
            "administrative" => Some(Self::Administrative),
            "identity_invalid" => Some(Self::IdentityInvalid),
            "user_requested" => Some(Self::UserRequested),
            _ => None,
        }
    }
}

/// Revocation list entry
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RevocationEntry {
    pub id: i64,
    /// Commitment hash or credential ID being revoked
    pub identifier: String,
    /// Type of identifier (commitment, credential_id, etc.)
    pub identifier_type: String,
    /// Reason for revocation
    pub reason: String,
    /// Timestamp of revocation
    pub revoked_at: String,
    /// Optional additional details
    pub details: Option<String>,
}

/// Revocation list manager
pub struct RevocationList {
    pool: Pool<Sqlite>,
}

impl RevocationList {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Initialize revocation list table
    pub async fn init_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS revocation_list (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                identifier TEXT NOT NULL UNIQUE,
                identifier_type TEXT NOT NULL,
                reason TEXT NOT NULL,
                revoked_at TEXT NOT NULL,
                details TEXT,
                UNIQUE(identifier, identifier_type)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for fast lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_revocation_identifier 
            ON revocation_list(identifier, identifier_type)
            "#,
        )
        .execute(&self.pool)
        .await?;

        info!("✅ Revocation list table initialized");
        Ok(())
    }

    /// Add an entry to the revocation list
    pub async fn revoke(
        &self,
        identifier: &str,
        identifier_type: &str,
        reason: RevocationReason,
        details: Option<&str>,
    ) -> Result<()> {
        let revoked_at = chrono::Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO revocation_list 
            (identifier, identifier_type, reason, revoked_at, details)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(identifier)
        .bind(identifier_type)
        .bind(reason.as_str())
        .bind(&revoked_at)
        .bind(details)
        .execute(&self.pool)
        .await?;

        warn!("🚫 REVOKED {}: {} (reason: {:?})", identifier_type, identifier, reason);
        Ok(())
    }

    /// Check if an identifier is revoked
    pub async fn is_revoked(&self, identifier: &str, identifier_type: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM revocation_list 
            WHERE identifier = ? AND identifier_type = ?
            "#,
        )
        .bind(identifier)
        .bind(identifier_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get revocation details for an identifier
    pub async fn get_revocation(&self, identifier: &str, identifier_type: &str) -> Result<Option<RevocationEntry>> {
        let entry = sqlx::query_as::<_, RevocationEntry>(
            r#"
            SELECT * FROM revocation_list 
            WHERE identifier = ? AND identifier_type = ?
            "#,
        )
        .bind(identifier)
        .bind(identifier_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Get all revocations (paginated)
    pub async fn list_revocations(&self, limit: i64, offset: i64) -> Result<Vec<RevocationEntry>> {
        let entries = sqlx::query_as::<_, RevocationEntry>(
            r#"
            SELECT * FROM revocation_list 
            ORDER BY revoked_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Count total revocations
    pub async fn count_revocations(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM revocation_list")
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    /// Get revocations by reason
    pub async fn list_by_reason(&self, reason: RevocationReason) -> Result<Vec<RevocationEntry>> {
        let entries = sqlx::query_as::<_, RevocationEntry>(
            r#"
            SELECT * FROM revocation_list 
            WHERE reason = ?
            ORDER BY revoked_at DESC
            "#,
        )
        .bind(reason.as_str())
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Batch check if multiple identifiers are revoked
    pub async fn check_batch(&self, identifiers: Vec<(String, String)>) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();
        
        for (identifier, id_type) in identifiers {
            let is_revoked = self.is_revoked(&identifier, &id_type).await?;
            results.push((identifier, is_revoked));
        }
        
        Ok(results)
    }

    /// Get statistics about revocations
    pub async fn get_statistics(&self) -> Result<RevocationStatistics> {
        let total = self.count_revocations().await?;
        
        let by_reason = sqlx::query_as::<_, (String, i64)>(
            "SELECT reason, COUNT(*) as count FROM revocation_list GROUP BY reason"
        )
        .fetch_all(&self.pool)
        .await?;

        let by_type = sqlx::query_as::<_, (String, i64)>(
            "SELECT identifier_type, COUNT(*) as count FROM revocation_list GROUP BY identifier_type"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(RevocationStatistics {
            total_revoked: total,
            by_reason: by_reason.into_iter().collect(),
            by_type: by_type.into_iter().collect(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevocationStatistics {
    pub total_revoked: i64,
    pub by_reason: Vec<(String, i64)>,
    pub by_type: Vec<(String, i64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_revocation_list_basic() {
        let pool = setup_test_db().await;
        let revlist = RevocationList::new(pool);
        
        revlist.init_table().await.unwrap();
        
        // Add revocation
        revlist.revoke(
            "test_hash_123",
            "commitment",
            RevocationReason::KeyCompromise,
            Some("Test compromise"),
        ).await.unwrap();
        
        // Check revocation
        assert!(revlist.is_revoked("test_hash_123", "commitment").await.unwrap());
        assert!(!revlist.is_revoked("other_hash", "commitment").await.unwrap());
        
        // Get details
        let entry = revlist.get_revocation("test_hash_123", "commitment").await.unwrap();
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.identifier, "test_hash_123");
        assert_eq!(entry.reason, "key_compromise");
        
        // Statistics
        let stats = revlist.get_statistics().await.unwrap();
        assert_eq!(stats.total_revoked, 1);
    }

    #[tokio::test]
    async fn test_batch_check() {
        let pool = setup_test_db().await;
        let revlist = RevocationList::new(pool);
        
        revlist.init_table().await.unwrap();
        
        revlist.revoke("hash1", "commitment", RevocationReason::Expired, None).await.unwrap();
        revlist.revoke("hash2", "commitment", RevocationReason::Administrative, None).await.unwrap();
        
        let results = revlist.check_batch(vec![
            ("hash1".to_string(), "commitment".to_string()),
            ("hash2".to_string(), "commitment".to_string()),
            ("hash3".to_string(), "commitment".to_string()),
        ]).await.unwrap();
        
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1, true);  // hash1 revoked
        assert_eq!(results[1].1, true);  // hash2 revoked
        assert_eq!(results[2].1, false); // hash3 not revoked
    }
}
