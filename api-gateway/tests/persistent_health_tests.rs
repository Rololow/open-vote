use anyhow::Result;
use axum_test::TestServer;
use serde_json::Value;
use api_gateway::persistent_gateway::PersistentApiGateway;

/// Sanity check: the persistent router exposes /health with expected payload
#[tokio::test]
async fn test_persistent_health_endpoint() -> Result<()> {
    // Arrange: in-memory persistent gateway for tests
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;

    // Act
    let response = server.get("/health").await;

    // Assert
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "api-gateway-persistent");
    assert!(body["timestamp"].is_string());

    Ok(())
}
