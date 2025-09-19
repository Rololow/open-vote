use axum::{
    http::{StatusCode, header::AUTHORIZATION},
};
use axum_test::TestServer;
use serde_json::{Value};

use api_gateway::{
    auth::{AuthManager, UserRole},
    routes::{LoginRequest, RegisterRequest, CreateLawRequest, VoteRequest, VoteType},
    gateway::create_router,
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

    let response = server.post("/auth/login")
        .json(&login_request)
        .await;

    response.assert_status_ok();
    let auth_response: Value = response.json();
    auth_response["token"].as_str().unwrap().to_string()
}

#[cfg(test)]
mod auth_routes_tests {
    use super::*;

    #[tokio::test]
    async fn test_login_with_valid_credentials() {
        let server = create_test_server().await;

        let login_request = LoginRequest {
            email: "marie@example.com".to_string(),
            password: "demo123".to_string(),
        };

        let response = server.post("/auth/login")
            .json(&login_request)
            .await;

        response.assert_status_ok();
        
        let auth_response: Value = response.json();
        assert!(auth_response["token"].is_string());
        assert_eq!(auth_response["user"]["email"], "marie@example.com");
        assert_eq!(auth_response["user"]["role"], "Citizen");
        assert!(auth_response["expires_in"].as_i64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_login_with_invalid_credentials() {
        let server = create_test_server().await;

        let login_request = LoginRequest {
            email: "marie@example.com".to_string(),
            password: "wrongpassword".to_string(),
        };

        let response = server.post("/auth/login")
            .json(&login_request)
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_register_new_user() {
        let server = create_test_server().await;

        let register_request = RegisterRequest {
            name: "Nouveau Utilisateur".to_string(),
            email: "nouveau@example.com".to_string(),
            password: "password123".to_string(),
            role: Some(UserRole::Citizen),
        };

        let response = server.post("/auth/register")
            .json(&register_request)
            .await;

        response.assert_status(StatusCode::CREATED);
        
        let auth_response: Value = response.json();
        assert!(auth_response["token"].is_string());
        assert_eq!(auth_response["user"]["email"], "nouveau@example.com");
        assert_eq!(auth_response["user"]["name"], "Nouveau Utilisateur");
        assert_eq!(auth_response["user"]["role"], "Citizen");
    }

    #[tokio::test]
    async fn test_register_duplicate_email() {
        let server = create_test_server().await;

        // Premier utilisateur
        let register_request1 = RegisterRequest {
            name: "Premier".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            role: Some(UserRole::Citizen),
        };

        let response1 = server.post("/auth/register")
            .json(&register_request1)
            .await;

        response1.assert_status(StatusCode::CREATED);

        // Tentative de dupliquer l'email
        let register_request2 = RegisterRequest {
            name: "Deuxième".to_string(),
            email: "test@example.com".to_string(),
            password: "password456".to_string(),
            role: Some(UserRole::Citizen),
        };

        let response2 = server.post("/auth/register")
            .json(&register_request2)
            .await;

        response2.assert_status(StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_profile_with_valid_token() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        let response = server.get("/auth/profile")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .await;

        response.assert_status_ok();
        
        let profile: Value = response.json();
        assert_eq!(profile["role"], "Citizen");
    }

    #[tokio::test]
    async fn test_profile_without_token() {
        let server = create_test_server().await;

        let response = server.get("/auth/profile")
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_profile_with_invalid_token() {
        let server = create_test_server().await;

        let response = server.get("/auth/profile")
            .add_header(AUTHORIZATION, "Bearer invalid_token")
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_logout() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        let response = server.post("/auth/logout")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .await;

        response.assert_status_ok();
        
        let logout_response: Value = response.json();
        assert_eq!(logout_response["message"], "Successfully logged out");
        assert!(logout_response["session_revoked"].is_string());
    }
}

#[cfg(test)]
mod law_routes_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_law_as_citizen() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        let create_request = CreateLawRequest {
            title: "Nouvelle Loi Test".to_string(),
            description: "Description de la loi de test".to_string(),
            content: "Contenu détaillé de la loi".to_string(),
            category: "Environnement".to_string(),
        };

        let response = server.post("/laws")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .json(&create_request)
            .await;

        response.assert_status(StatusCode::CREATED);
        
        let law: Value = response.json();
        assert_eq!(law["title"], "Nouvelle Loi Test");
        assert_eq!(law["description"], "Description de la loi de test");
        assert_eq!(law["category"], "Environnement");
        assert_eq!(law["status"], "Draft");
        assert!(law["id"].is_string());
    }

    #[tokio::test]
    async fn test_create_law_without_auth() {
        let server = create_test_server().await;

        let create_request = CreateLawRequest {
            title: "Loi Non Autorisée".to_string(),
            description: "Cette création devrait échouer".to_string(),
            content: "Contenu".to_string(),
            category: "Test".to_string(),
        };

        let response = server.post("/laws")
            .json(&create_request)
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_list_laws() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        // Créer d'abord une loi
        let create_request = CreateLawRequest {
            title: "Loi pour Listing".to_string(),
            description: "Test de listing".to_string(),
            content: "Contenu".to_string(),
            category: "Test".to_string(),
        };

        server.post("/laws")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .json(&create_request)
            .await;

        // Lister les lois
        let response = server.get("/laws")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .await;

        response.assert_status_ok();
        
        let laws: Value = response.json();
        assert!(laws.is_array());
        let laws_array = laws.as_array().unwrap();
        assert!(!laws_array.is_empty());
        
        let first_law = &laws_array[0];
        assert_eq!(first_law["title"], "Loi pour Listing");
    }

    #[tokio::test]
    async fn test_get_specific_law() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        // Créer une loi
        let create_request = CreateLawRequest {
            title: "Loi Spécifique".to_string(),
            description: "Pour test get".to_string(),
            content: "Contenu spécifique".to_string(),
            category: "Test".to_string(),
        };

        let create_response = server.post("/laws")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .json(&create_request)
            .await;

        let created_law: Value = create_response.json();
        let law_id = created_law["id"].as_str().unwrap();

        // Récupérer la loi spécifique
        let response = server.get(&format!("/laws/{}", law_id))
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .await;

        response.assert_status_ok();
        
        let law: Value = response.json();
        assert_eq!(law["title"], "Loi Spécifique");
        assert_eq!(law["id"], law_id);
    }

    #[tokio::test]
    async fn test_get_nonexistent_law() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        let response = server.get("/laws/nonexistent-id")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .await;

        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_vote_on_law() {
        let server = create_test_server().await;
        let token = login_user(&server, "marie@example.com", "demo123").await;

        // Créer une loi
        let create_request = CreateLawRequest {
            title: "Loi pour Vote".to_string(),
            description: "Pour tester le vote".to_string(),
            content: "Contenu".to_string(),
            category: "Test".to_string(),
        };

        let create_response = server.post("/laws")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .json(&create_request)
            .await;

        let created_law: Value = create_response.json();
        let law_id = created_law["id"].as_str().unwrap().to_string();

        // Voter pour la loi
        let vote_request = VoteRequest {
            law_id: law_id.clone(),
            vote_type: VoteType::For,
            comment: Some("Je soutiens cette loi".to_string()),
        };

        let response = server.post("/vote")
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .json(&vote_request)
            .await;

        response.assert_status_ok();
        
        let vote_response: Value = response.json();
        assert_eq!(vote_response["message"], "Vote recorded successfully");
        assert_eq!(vote_response["law_id"], law_id);
        assert_eq!(vote_response["vote"], "For");
        assert_eq!(vote_response["current_votes"]["for"], 1);
    }
}

#[cfg(test)]
mod admin_routes_tests {
    use super::*;

    #[tokio::test]
    async fn test_admin_list_users() {
        let server = create_test_server().await;
        let admin_token = login_user(&server, "admin@example.com", "admin123").await;

        let response = server.get("/admin/users")
            .add_header(AUTHORIZATION, format!("Bearer {}", admin_token))
            .await;

        response.assert_status_ok();
        
        let users: Value = response.json();
        assert!(users.is_array());
        // Au moins l'admin devrait être présent
        assert!(!users.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_non_admin_cannot_list_users() {
        let server = create_test_server().await;
        let citizen_token = login_user(&server, "marie@example.com", "demo123").await;

        let response = server.get("/admin/users")
            .add_header(AUTHORIZATION, format!("Bearer {}", citizen_token))
            .await;

        response.assert_status(StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_get_stats_as_admin() {
        let server = create_test_server().await;
        let admin_token = login_user(&server, "admin@example.com", "admin123").await;

        let response = server.get("/stats")
            .add_header(AUTHORIZATION, format!("Bearer {}", admin_token))
            .await;

        response.assert_status_ok();
        
        let stats: Value = response.json();
        assert!(stats["users_count"].is_number());
        assert!(stats["laws_count"].is_number());
        assert!(stats["laws_by_status"].is_object());
        assert!(stats["total_votes"].is_number());
    }

    #[tokio::test]
    async fn test_citizen_cannot_access_stats() {
        let server = create_test_server().await;
        let citizen_token = login_user(&server, "marie@example.com", "demo123").await;

        let response = server.get("/stats")
            .add_header(AUTHORIZATION, format!("Bearer {}", citizen_token))
            .await;

        response.assert_status(StatusCode::FORBIDDEN);
    }
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[tokio::test]
    async fn test_representative_can_modify_law() {
        let server = create_test_server().await;
        let rep_token = login_user(&server, "jean@example.com", "demo123").await;

        // Créer une loi en tant que représentant
        let create_request = CreateLawRequest {
            title: "Loi Modifiable".to_string(),
            description: "Pour test de modification".to_string(),
            content: "Contenu original".to_string(),
            category: "Test".to_string(),
        };

        let create_response = server.post("/laws")
            .add_header(AUTHORIZATION, format!("Bearer {}", rep_token))
            .json(&create_request)
            .await;

        let created_law: Value = create_response.json();
        let law_id = created_law["id"].as_str().unwrap();

        // Modifier la loi
        let update_request = CreateLawRequest {
            title: "Loi Modifiée".to_string(),
            description: "Description modifiée".to_string(),
            content: "Contenu modifié".to_string(),
            category: "Test Modifié".to_string(),
        };

        let response = server.put(&format!("/laws/{}", law_id))
            .add_header(AUTHORIZATION, format!("Bearer {}", rep_token))
            .json(&update_request)
            .await;

        response.assert_status_ok();
        
        let updated_law: Value = response.json();
        assert_eq!(updated_law["title"], "Loi Modifiée");
        assert_eq!(updated_law["description"], "Description modifiée");
    }

    #[tokio::test]
    async fn test_citizen_cannot_modify_law_without_permission() {
        let server = create_test_server().await;
        let citizen_token = login_user(&server, "marie@example.com", "demo123").await;

        // Un citoyen ne peut pas modifier les lois des autres
        let update_request = CreateLawRequest {
            title: "Tentative de Modification".to_string(),
            description: "Ne devrait pas marcher".to_string(),
            content: "Contenu".to_string(),
            category: "Test".to_string(),
        };

        let response = server.put("/laws/fake-id")
            .add_header(AUTHORIZATION, format!("Bearer {}", citizen_token))
            .json(&update_request)
            .await;

        response.assert_status(StatusCode::FORBIDDEN);
    }
}