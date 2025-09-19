use axum::http::{StatusCode, header};
use axum_test::TestServer;
use serde_json::Value;

use api_gateway::{
    auth::{AuthManager, UserRole},
    routes::{LoginRequest, RegisterRequest, create_router},
};
use crypto_lib::KeyPair;

/// Helper pour créer un serveur de test
async fn create_test_server() -> TestServer {
    let signing_key = KeyPair::generate();
    let auth_manager = AuthManager::new(signing_key, 24);
    let app = create_router(auth_manager);
    
    TestServer::new(app).unwrap()
}

/// Helper pour s'authentifier et récupérer un token
async fn login_user(server: &TestServer, email: &str, password: &str) -> String {
    let login_request = LoginRequest {
        email: email.to_string(),
        password: password.to_string(),
    };

    let response = server.post("/api/auth/login")
        .json(&login_request)
        .await;

    response.assert_status_ok();
    let auth_response: Value = response.json();
    auth_response["token"].as_str().unwrap().to_string()
}

#[cfg(test)]
mod simple_auth_tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        let server = create_test_server().await;
        
        let response = server.get("/health").await;
        response.assert_status_ok();
        
        let health: Value = response.json();
        assert_eq!(health["status"], "healthy");
        assert_eq!(health["service"], "api-gateway");
    }

    #[tokio::test]
    async fn test_register_new_user() {
        let server = create_test_server().await;

        let register_request = RegisterRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            role: Some(UserRole::Citizen),
        };

        let response = server.post("/api/auth/register")
            .json(&register_request)
            .await;

        response.assert_status(StatusCode::CREATED);
        
        let auth_response: Value = response.json();
        assert!(auth_response["token"].is_string());
        assert_eq!(auth_response["user"]["email"], "test@example.com");
        assert_eq!(auth_response["user"]["role"], "Citizen");
    }

    #[tokio::test]
    async fn test_login_after_registration() {
        let server = create_test_server().await;

        // Enregistrer un utilisateur
        let register_request = RegisterRequest {
            name: "Login Test User".to_string(),
            email: "logintest@example.com".to_string(),
            password: "testpass123".to_string(),
            role: Some(UserRole::Citizen),
        };

        server.post("/api/auth/register")
            .json(&register_request)
            .await
            .assert_status(StatusCode::CREATED);

        // Se connecter avec les mêmes identifiants
        let login_request = LoginRequest {
            email: "logintest@example.com".to_string(),
            password: "testpass123".to_string(),
        };

        let response = server.post("/api/auth/login")
            .json(&login_request)
            .await;

        response.assert_status_ok();
        
        let auth_response: Value = response.json();
        assert!(auth_response["token"].is_string());
        assert_eq!(auth_response["user"]["email"], "logintest@example.com");
    }

    #[tokio::test]
    async fn test_login_with_wrong_credentials() {
        let server = create_test_server().await;

        let login_request = LoginRequest {
            email: "nonexistent@example.com".to_string(),
            password: "wrongpassword".to_string(),
        };

        let response = server.post("/api/auth/login")
            .json(&login_request)
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_protected_route_without_token() {
        let server = create_test_server().await;

        let response = server.get("/api/auth/profile").await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test] 
    async fn test_protected_route_with_token() {
        let server = create_test_server().await;

        // Enregistrer et récupérer un token
        let register_request = RegisterRequest {
            name: "Profile Test User".to_string(),
            email: "profile@example.com".to_string(),
            password: "profilepass123".to_string(),
            role: Some(UserRole::Citizen),
        };

        let register_response = server.post("/api/auth/register")
            .json(&register_request)
            .await;

        register_response.assert_status(StatusCode::CREATED);
        
        let auth_response: Value = register_response.json();
        let token = auth_response["token"].as_str().unwrap();

        // Accéder au profil avec le token
        let response = server.get("/api/auth/profile")
            .add_header(
                header::AUTHORIZATION,
                format!("Bearer {}", token)
            )
            .await;

        response.assert_status_ok();
        
        let profile: Value = response.json();
        assert_eq!(profile["email"], "profile@example.com");
        assert_eq!(profile["role"], "Citizen");
    }
}