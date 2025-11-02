use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn, info};

/// Security event types for monitoring (Phase 6.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEvent {
    /// Verification failure (ZKP, signature, etc.)
    VerificationFailure {
        event_type: String,
        identifier: String,
        reason: String,
    },
    /// Parameter mismatch detected
    ParameterMismatch {
        parameter_name: String,
        expected: String,
        actual: String,
    },
    /// Revocation detected
    RevocationDetected {
        identifier: String,
        identifier_type: String,
    },
    /// Key rotation event
    KeyRotation {
        from_version: u32,
        to_version: u32,
    },
    /// Potential security incident
    SecurityIncident {
        severity: String,
        description: String,
    },
    /// Unusual activity pattern
    AnomalousActivity {
        activity_type: String,
        description: String,
    },
}

/// Security event record for database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SecurityEventRecord {
    pub id: i64,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub metadata: Option<String>,
    pub timestamp: String,
}

/// Monitoring metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitoringMetrics {
    /// Total verification attempts
    pub total_verifications: u64,
    /// Failed verifications
    pub failed_verifications: u64,
    /// Parameter mismatches detected
    pub parameter_mismatches: u64,
    /// Revocations processed
    pub revocations_processed: u64,
    /// Security incidents
    pub security_incidents: u64,
    /// Last reset timestamp
    pub last_reset: Option<String>,
}

/// Security monitoring service
pub struct SecurityMonitor {
    pool: Pool<Sqlite>,
    metrics: Arc<RwLock<MonitoringMetrics>>,
}

impl SecurityMonitor {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self {
            pool,
            metrics: Arc::new(RwLock::new(MonitoringMetrics::default())),
        }
    }

    /// Initialize monitoring tables
    pub async fn init_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS security_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                description TEXT NOT NULL,
                metadata TEXT,
                timestamp TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for efficient queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_security_events_timestamp 
            ON security_events(timestamp DESC)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_security_events_severity 
            ON security_events(severity, timestamp DESC)
            "#,
        )
        .execute(&self.pool)
        .await?;

        info!("✅ Security monitoring tables initialized");
        Ok(())
    }

    /// Log a security event
    pub async fn log_event(&self, event: SecurityEvent) -> Result<()> {
        let (event_type, severity, description, metadata) = match &event {
            SecurityEvent::VerificationFailure { event_type, identifier, reason } => {
                let desc = format!("Verification failed for {}: {}", identifier, reason);
                let meta = serde_json::json!({
                    "event_type": event_type,
                    "identifier": identifier,
                    "reason": reason,
                });
                ("verification_failure", "medium", desc, Some(meta.to_string()))
            }
            SecurityEvent::ParameterMismatch { parameter_name, expected, actual } => {
                let desc = format!("Parameter mismatch: {} (expected: {}, actual: {})", 
                                   parameter_name, expected, actual);
                let meta = serde_json::json!({
                    "parameter": parameter_name,
                    "expected": expected,
                    "actual": actual,
                });
                ("parameter_mismatch", "high", desc, Some(meta.to_string()))
            }
            SecurityEvent::RevocationDetected { identifier, identifier_type } => {
                let desc = format!("Revocation detected: {} ({})", identifier, identifier_type);
                let meta = serde_json::json!({
                    "identifier": identifier,
                    "type": identifier_type,
                });
                ("revocation_detected", "low", desc, Some(meta.to_string()))
            }
            SecurityEvent::KeyRotation { from_version, to_version } => {
                let desc = format!("Key rotation: v{} -> v{}", from_version, to_version);
                let meta = serde_json::json!({
                    "from_version": from_version,
                    "to_version": to_version,
                });
                ("key_rotation", "low", desc, Some(meta.to_string()))
            }
            SecurityEvent::SecurityIncident { severity, description } => {
                let meta = serde_json::json!({
                    "severity": severity,
                });
                ("security_incident", severity.as_str(), description.clone(), Some(meta.to_string()))
            }
            SecurityEvent::AnomalousActivity { activity_type, description } => {
                let meta = serde_json::json!({
                    "activity_type": activity_type,
                });
                ("anomalous_activity", "medium", description.clone(), Some(meta.to_string()))
            }
        };

        let timestamp = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO security_events (event_type, severity, description, metadata, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(event_type)
        .bind(severity)
        .bind(&description)
        .bind(metadata)
        .bind(&timestamp)
        .execute(&self.pool)
        .await?;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        match event {
            SecurityEvent::VerificationFailure { .. } => {
                metrics.failed_verifications += 1;
                warn!("⚠️ {}", description);
            }
            SecurityEvent::ParameterMismatch { .. } => {
                metrics.parameter_mismatches += 1;
                error!("🚨 {}", description);
            }
            SecurityEvent::RevocationDetected { .. } => {
                metrics.revocations_processed += 1;
                info!("📋 {}", description);
            }
            SecurityEvent::SecurityIncident { .. } => {
                metrics.security_incidents += 1;
                error!("🚨 INCIDENT: {}", description);
            }
            _ => {
                info!("📊 {}", description);
            }
        }

        Ok(())
    }

    /// Record a verification attempt
    pub async fn record_verification(&self, success: bool, event_type: &str, identifier: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.total_verifications += 1;
        
        if !success {
            metrics.failed_verifications += 1;
            
            // Log the failure
            if let Err(e) = self.log_event(SecurityEvent::VerificationFailure {
                event_type: event_type.to_string(),
                identifier: identifier.to_string(),
                reason: "Verification failed".to_string(),
            }).await {
                error!("Failed to log verification failure: {}", e);
            }
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> MonitoringMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset metrics (for periodic reports)
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = MonitoringMetrics {
            last_reset: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        };
        info!("📊 Monitoring metrics reset");
    }

    /// Get recent security events
    pub async fn get_recent_events(&self, limit: i64) -> Result<Vec<SecurityEventRecord>> {
        let events = sqlx::query_as::<_, SecurityEventRecord>(
            r#"
            SELECT * FROM security_events 
            ORDER BY timestamp DESC 
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get events by severity
    pub async fn get_events_by_severity(&self, severity: &str, limit: i64) -> Result<Vec<SecurityEventRecord>> {
        let events = sqlx::query_as::<_, SecurityEventRecord>(
            r#"
            SELECT * FROM security_events 
            WHERE severity = ?
            ORDER BY timestamp DESC 
            LIMIT ?
            "#,
        )
        .bind(severity)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Count events by type in time period
    pub async fn count_events_since(&self, since: &str) -> Result<Vec<(String, i64)>> {
        let counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT event_type, COUNT(*) as count 
            FROM security_events 
            WHERE timestamp >= ?
            GROUP BY event_type
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        Ok(counts)
    }

    /// Generate security report
    pub async fn generate_report(&self, hours: i64) -> Result<SecurityReport> {
        let since = chrono::Utc::now() - chrono::Duration::hours(hours);
        let since_str = since.to_rfc3339();

        let event_counts = self.count_events_since(&since_str).await?;
        let high_severity = self.get_events_by_severity("high", 100).await?;
        let metrics = self.get_metrics().await;

        Ok(SecurityReport {
            period_hours: hours,
            period_start: since_str,
            period_end: chrono::Utc::now().to_rfc3339(),
            event_counts,
            high_severity_events: high_severity.len() as i64,
            metrics,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub period_hours: i64,
    pub period_start: String,
    pub period_end: String,
    pub event_counts: Vec<(String, i64)>,
    pub high_severity_events: i64,
    pub metrics: MonitoringMetrics,
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
    async fn test_security_monitoring() {
        let pool = setup_test_db().await;
        let monitor = SecurityMonitor::new(pool);
        
        monitor.init_tables().await.unwrap();
        
        // Log verification failure
        monitor.log_event(SecurityEvent::VerificationFailure {
            event_type: "zkp".to_string(),
            identifier: "test_id".to_string(),
            reason: "Invalid proof".to_string(),
        }).await.unwrap();
        
        // Log parameter mismatch
        monitor.log_event(SecurityEvent::ParameterMismatch {
            parameter_name: "merkle_root".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        }).await.unwrap();
        
        // Check metrics
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.failed_verifications, 1);
        assert_eq!(metrics.parameter_mismatches, 1);
        
        // Get recent events
        let events = monitor.get_recent_events(10).await.unwrap();
        assert_eq!(events.len(), 2);
        
        // Generate report
        let report = monitor.generate_report(24).await.unwrap();
        assert!(report.event_counts.len() > 0);
    }
}
