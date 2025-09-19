use anyhow::Result;
use axum_test::TestServer;
use axum::http::StatusCode;
use serde_json::{json, Value};
use api_gateway::persistent_gateway::PersistentApiGateway;

/// Happy path: register → login → profile on persistent API (in-memory SQLite)
#[tokio::test]
async fn test_persistent_register_login_profile() -> Result<()> {
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;

    // Register a new user
    let email = "smoke.user@example.com";
    let password = "SmokeTestPass123!";
    let register = json!({
        "email": email,
        "name": "Smoke User",
        "password": password
    });

    let register_resp = server
        .post("/api/v2/auth/register")
        .json(&register)
        .await;
    register_resp.assert_status(StatusCode::CREATED);

    // Login with same credentials
    let login = json!({
        "email": email,
        "password": password,
    });

    let login_resp = server
        .post("/api/v2/auth/login")
        .json(&login)
        .await;
    login_resp.assert_status_ok();
    let login_body: Value = login_resp.json();
    assert_eq!(login_body["success"], true);
    let token = login_body["data"]["token"].as_str().expect("missing token");

    // Fetch profile
    let profile_resp = server
        .get("/api/v2/auth/profile")
        .add_header(
            "Authorization",
            &format!("Bearer {}", token),
        )
        .await;

    profile_resp.assert_status_ok();
    let profile: Value = profile_resp.json();
    assert_eq!(profile["success"], true);
    assert_eq!(profile["data"]["email"], email);

    Ok(())
}
