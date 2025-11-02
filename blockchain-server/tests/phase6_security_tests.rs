use blockchain_server::{
    issuer::rotation::KeyRotationConfig,
    revocation::{RevocationList, RevocationReason},
    monitoring::{SecurityMonitor, SecurityEvent},
};
use sqlx::sqlite::SqlitePoolOptions;
use std::env;

/// Test complete Phase 6 security workflow
#[tokio::test]
async fn test_phase6_complete_workflow() {
    // Setup test database
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Initialize revocation list
    let revlist = RevocationList::new(pool.clone());
    revlist.init_table().await.expect("Failed to init revocation table");

    // Initialize monitoring
    let monitor = SecurityMonitor::new(pool.clone());
    monitor.init_tables().await.expect("Failed to init monitoring tables");

    // Test 1: Verify no revocations initially
    let is_revoked = revlist.is_revoked("test_commitment", "commitment").await.unwrap();
    assert!(!is_revoked, "Should not be revoked initially");

    // Test 2: Add a revocation
    revlist.revoke(
        "test_commitment",
        "commitment",
        RevocationReason::KeyCompromise,
        Some("Test key compromise"),
    ).await.expect("Failed to revoke");

    // Test 3: Verify revocation is active
    let is_revoked = revlist.is_revoked("test_commitment", "commitment").await.unwrap();
    assert!(is_revoked, "Should be revoked after adding to list");

    // Test 4: Log security events
    monitor.log_event(SecurityEvent::VerificationFailure {
        event_type: "zkp".to_string(),
        identifier: "test_id".to_string(),
        reason: "Invalid proof".to_string(),
    }).await.expect("Failed to log event");

    monitor.log_event(SecurityEvent::ParameterMismatch {
        parameter_name: "merkle_root".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    }).await.expect("Failed to log parameter mismatch");

    // Test 5: Verify metrics updated
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.failed_verifications, 1, "Should have 1 failed verification");
    assert_eq!(metrics.parameter_mismatches, 1, "Should have 1 parameter mismatch");

    // Test 6: Get recent events
    let events = monitor.get_recent_events(10).await.expect("Failed to get events");
    assert_eq!(events.len(), 2, "Should have 2 logged events");

    // Test 7: Generate security report
    let report = monitor.generate_report(24).await.expect("Failed to generate report");
    assert!(report.event_counts.len() > 0, "Report should have event counts");

    // Test 8: Revocation statistics
    let stats = revlist.get_statistics().await.expect("Failed to get statistics");
    assert_eq!(stats.total_revoked, 1, "Should have 1 total revocation");
}

/// Test key rotation configuration
#[tokio::test]
async fn test_key_rotation_config() {
    let tmp_dir = env::temp_dir();
    let config_path = tmp_dir.join("phase6_rotation_test.json");
    
    // Clean up before test
    let _ = std::fs::remove_file(&config_path);
    
    // Create rotation config
    let mut config = KeyRotationConfig::load_or_create(&config_path)
        .expect("Failed to create rotation config");
    
    // Add version 1 key
    config.add_verification_key(
        1,
        "aabbccdd".to_string(),
        "did:key:z6MkTest1".to_string(),
    ).expect("Failed to add VK v1");
    config.active_version = 1;
    
    // Add version 2 key
    config.add_verification_key(
        2,
        "eeffgghh".to_string(),
        "did:key:z6MkTest2".to_string(),
    ).expect("Failed to add VK v2");
    
    // Test rotation
    config.rotate_to_version(2).expect("Failed to rotate to v2");
    assert_eq!(config.active_version, 2, "Active version should be 2");
    
    // Verify both keys are trusted
    assert!(config.is_version_trusted(1), "Version 1 should still be trusted");
    assert!(config.is_version_trusted(2), "Version 2 should be trusted");
    
    // Get trusted VKs
    let trusted = config.get_trusted_vks();
    assert_eq!(trusted.len(), 2, "Should have 2 trusted VKs");
    
    // Test revocation
    config.revoke_version(1, "Compromise detected").expect("Failed to revoke v1");
    assert!(!config.is_version_trusted(1), "Version 1 should not be trusted after revoke");
    assert!(config.is_version_trusted(2), "Version 2 should still be trusted");
    
    // Verify only 1 trusted VK remains
    let trusted = config.get_trusted_vks();
    assert_eq!(trusted.len(), 1, "Should have 1 trusted VK after revocation");
    
    // Test persistence
    config.save(&config_path).expect("Failed to save config");
    
    // Reload and verify
    let reloaded = KeyRotationConfig::load_or_create(&config_path)
        .expect("Failed to reload config");
    assert_eq!(reloaded.active_version, 2, "Active version should persist");
    assert!(!reloaded.is_version_trusted(1), "Revocation should persist");
    assert!(reloaded.is_version_trusted(2), "Trust should persist");
    
    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

/// Test monitoring with multiple event types
#[tokio::test]
async fn test_monitoring_event_filtering() {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    let monitor = SecurityMonitor::new(pool.clone());
    monitor.init_tables().await.expect("Failed to init tables");

    // Log various events
    monitor.log_event(SecurityEvent::KeyRotation {
        from_version: 1,
        to_version: 2,
    }).await.expect("Failed to log rotation");

    monitor.log_event(SecurityEvent::SecurityIncident {
        severity: "high".to_string(),
        description: "Test incident".to_string(),
    }).await.expect("Failed to log incident");

    monitor.log_event(SecurityEvent::AnomalousActivity {
        activity_type: "unusual_pattern".to_string(),
        description: "Test anomaly".to_string(),
    }).await.expect("Failed to log anomaly");

    // Get high severity events
    let high_events = monitor.get_events_by_severity("high", 10).await.expect("Failed to get high events");
    assert_eq!(high_events.len(), 1, "Should have 1 high severity event");
    assert_eq!(high_events[0].severity, "high");

    // Get all events
    let all_events = monitor.get_recent_events(10).await.expect("Failed to get all events");
    assert_eq!(all_events.len(), 3, "Should have 3 total events");

    // Verify metrics
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.security_incidents, 1, "Should have 1 security incident");
}

/// Test batch revocation checking
#[tokio::test]
async fn test_batch_revocation_check() {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    let revlist = RevocationList::new(pool.clone());
    revlist.init_table().await.expect("Failed to init table");

    // Add multiple revocations
    revlist.revoke("hash1", "commitment", RevocationReason::Expired, None).await.unwrap();
    revlist.revoke("hash2", "commitment", RevocationReason::Administrative, None).await.unwrap();
    revlist.revoke("hash3", "credential", RevocationReason::KeyCompromise, None).await.unwrap();

    // Batch check
    let to_check = vec![
        ("hash1".to_string(), "commitment".to_string()),
        ("hash2".to_string(), "commitment".to_string()),
        ("hash3".to_string(), "credential".to_string()),
        ("hash4".to_string(), "commitment".to_string()),
    ];

    let results = revlist.check_batch(to_check).await.expect("Failed to batch check");
    
    assert_eq!(results.len(), 4, "Should check 4 items");
    assert_eq!(results[0].1, true, "hash1 should be revoked");
    assert_eq!(results[1].1, true, "hash2 should be revoked");
    assert_eq!(results[2].1, true, "hash3 should be revoked");
    assert_eq!(results[3].1, false, "hash4 should not be revoked");
}

/// Test revocation by reason filtering
#[tokio::test]
async fn test_revocation_by_reason() {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    let revlist = RevocationList::new(pool.clone());
    revlist.init_table().await.expect("Failed to init table");

    // Add revocations with different reasons
    revlist.revoke("c1", "commitment", RevocationReason::KeyCompromise, None).await.unwrap();
    revlist.revoke("c2", "commitment", RevocationReason::KeyCompromise, None).await.unwrap();
    revlist.revoke("c3", "commitment", RevocationReason::Expired, None).await.unwrap();
    revlist.revoke("c4", "credential", RevocationReason::Administrative, None).await.unwrap();

    // Get by reason
    let compromised = revlist.list_by_reason(RevocationReason::KeyCompromise).await.unwrap();
    assert_eq!(compromised.len(), 2, "Should have 2 key compromise revocations");

    let expired = revlist.list_by_reason(RevocationReason::Expired).await.unwrap();
    assert_eq!(expired.len(), 1, "Should have 1 expired revocation");

    let admin = revlist.list_by_reason(RevocationReason::Administrative).await.unwrap();
    assert_eq!(admin.len(), 1, "Should have 1 administrative revocation");

    // Get statistics
    let stats = revlist.get_statistics().await.unwrap();
    assert_eq!(stats.total_revoked, 4, "Should have 4 total revocations");
    
    // Verify reason breakdown
    let key_compromise_count = stats.by_reason.iter()
        .find(|(reason, _)| reason == "key_compromise")
        .map(|(_, count)| *count)
        .unwrap_or(0);
    assert_eq!(key_compromise_count, 2, "Should have 2 key compromise in stats");
}

/// Integration test: Complete security scenario
#[tokio::test]
async fn test_complete_security_scenario() {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    let revlist = RevocationList::new(pool.clone());
    let monitor = SecurityMonitor::new(pool.clone());
    
    revlist.init_table().await.expect("Failed to init revocation");
    monitor.init_tables().await.expect("Failed to init monitoring");

    // Scenario: Detect compromised credential
    let credential_id = "cred_12345";
    
    // Step 1: Verification fails multiple times
    monitor.record_verification(false, "zkp", credential_id).await;
    monitor.record_verification(false, "zkp", credential_id).await;
    monitor.record_verification(false, "zkp", credential_id).await;
    
    // Step 2: Parameter mismatch detected
    monitor.log_event(SecurityEvent::ParameterMismatch {
        parameter_name: "nullifier".to_string(),
        expected: "expected_value".to_string(),
        actual: "wrong_value".to_string(),
    }).await.expect("Failed to log mismatch");
    
    // Step 3: Revoke the credential
    revlist.revoke(
        credential_id,
        "credential",
        RevocationReason::KeyCompromise,
        Some("Multiple verification failures and parameter mismatch"),
    ).await.expect("Failed to revoke");
    
    // Step 4: Log revocation event
    monitor.log_event(SecurityEvent::RevocationDetected {
        identifier: credential_id.to_string(),
        identifier_type: "credential".to_string(),
    }).await.expect("Failed to log revocation");
    
    // Step 5: Verify state
    assert!(revlist.is_revoked(credential_id, "credential").await.unwrap(), 
            "Credential should be revoked");
    
    let metrics = monitor.get_metrics().await;
    assert_eq!(metrics.failed_verifications, 3, "Should have 3 failed verifications");
    assert_eq!(metrics.parameter_mismatches, 1, "Should have 1 parameter mismatch");
    assert_eq!(metrics.revocations_processed, 1, "Should have processed 1 revocation");
    
    // Step 6: Generate incident report
    let report = monitor.generate_report(1).await.expect("Failed to generate report");
    assert!(report.high_severity_events > 0, "Should have high severity events");
    assert!(report.event_counts.len() > 0, "Should have event counts");
}
