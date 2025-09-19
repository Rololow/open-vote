use axum::{
    extract::{Path, Query, State}, 
    http::{StatusCode, Method}, 
    response::{Json, IntoResponse}, 
    routing::{get, post, put, delete}, 
    Router, middleware
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{auth::{AuthManager, UserRole}, middleware::auth_middleware};
use crypto_lib;

/// État partagé de l'application
#[derive(Clone)]
pub struct AppState {
    pub auth_state: crate::middleware::AuthState,
}

impl AppState {
    pub fn new(auth_manager: AuthManager) -> Self {
        Self {
            auth_state: crate::middleware::AuthState {
                auth_manager: Arc::new(RwLock::new(auth_manager)),
            },
        }
    }
}

/// Structure pour les requêtes de connexion
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Structure pour les requêtes d'inscription
#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: Option<UserRole>,
}

/// Structure pour les réponses d'authentification
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
    pub expires_in: i64,
}

/// Informations utilisateur publiques
#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub permissions: Vec<String>,
    pub reputation: Option<u32>,
}

/// Structure pour les requêtes de création de lois
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateLawRequest {
    pub title: String,
    pub description: String,
    pub content: String,
    pub category: String,
}

/// Structure pour les requêtes de vote
#[derive(Debug, Deserialize, Serialize)]
pub struct VoteRequest {
    pub law_id: String,
    pub vote_type: VoteType,
    pub comment: Option<String>,
}

/// Types de vote
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

// =============================================================================
// HANDLERS D'AUTHENTIFICATION
// =============================================================================

/// Connexion utilisateur
pub async fn login(
    State(app_state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    // Pour la démo, on accepte quelques comptes prédéfinis
    let demo_users = [
        ("marie@example.com", "demo123", UserRole::Citizen),
        ("jean@example.com", "demo123", UserRole::Government),
        ("admin@example.com", "admin123", UserRole::Administrator),
    ];

    // Vérifier les identifiants
    let user_data = demo_users.iter()
        .find(|(email, password, _)| *email == request.email && *password == request.password);

    if let Some((email, _, role)) = user_data {
        // Générer un token JWT
        let auth_state = app_state.auth_state;
        let user_id = Uuid::new_v4();
        let keypair = crypto_lib::KeyPair::generate();
        let public_key = keypair.public_key().clone();
        let permissions = role.default_permissions();
        
        let token_result = auth_state.auth_manager.write().await
            .generate_token(user_id, public_key, role.clone(), permissions);

        match token_result {
            Ok(token) => {
                let user_info = UserInfo {
                    id: Uuid::new_v4().to_string(),
                    name: email.split('@').next().unwrap_or("User").to_string(),
                    email: email.to_string(),
                    role: role.clone(),
                    permissions: role.default_permissions(),
                    reputation: Some(100),
                };

                let response = AuthResponse {
                    token: token.encode().unwrap_or_default(),
                    user: user_info,
                    expires_in: 24 * 3600, // 24 heures
                };

                Ok(Json(response))
            }
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Inscription d'un nouvel utilisateur
pub async fn register(
    State(app_state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), StatusCode> {
    // Pour la démo, on accepte toutes les inscriptions
    let role = request.role.unwrap_or(UserRole::Citizen);
    
    // Générer un token JWT
    let auth_state = app_state.auth_state;
    let user_id = Uuid::new_v4();
    let keypair = crypto_lib::KeyPair::generate();
    let public_key = keypair.public_key().clone();
    let permissions = role.default_permissions();
    
    let token_result = auth_state.auth_manager.write().await
        .generate_token(user_id, public_key, role.clone(), permissions);

    match token_result {
        Ok(token) => {
            let user_info = UserInfo {
                id: Uuid::new_v4().to_string(),
                name: request.name,
                email: request.email,
                role: role.clone(),
                permissions: role.default_permissions(),
                reputation: Some(50), // Réputation initiale plus faible
            };

            let response = AuthResponse {
                token: token.encode().unwrap_or_default(),
                user: user_info,
                expires_in: 24 * 3600,
            };

            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Profil utilisateur (route protégée)
pub async fn profile(
    crate::middleware::AuthenticatedUser { claims }: crate::middleware::AuthenticatedUser,
) -> Json<UserInfo> {
    Json(UserInfo {
        id: claims.user_id().unwrap_or_default().to_string(),
        name: claims.sub.split('@').next().unwrap_or("User").to_string(),
        email: claims.sub.clone(),
        role: claims.role.clone(),
        permissions: claims.role.default_permissions(),
        reputation: Some(100),
    })
}

/// Déconnexion (route protégée)
pub async fn logout() -> StatusCode {
    // Pour une vraie application, on invaliderait le token ici
    StatusCode::OK
}

// =============================================================================
// HANDLERS POUR LES LOIS
// =============================================================================

/// Lister toutes les lois
pub async fn list_laws() -> Json<Vec<serde_json::Value>> {
    Json(vec![
        json!({
            "id": "law-1",
            "title": "Loi de protection des données",
            "status": "active",
            "votes": {"for": 45, "against": 12, "abstain": 3}
        }),
        json!({
            "id": "law-2", 
            "title": "Réforme du système de santé",
            "status": "draft",
            "votes": {"for": 23, "against": 18, "abstain": 7}
        })
    ])
}

/// Obtenir une loi spécifique
pub async fn get_law(Path(id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({
        "id": id,
        "title": "Loi exemple",
        "description": "Description de la loi",
        "content": "Contenu détaillé de la loi...",
        "status": "active",
        "votes": {"for": 45, "against": 12, "abstain": 3},
        "created_at": "2024-01-15T10:00:00Z"
    }))
}

/// Créer une nouvelle loi (nécessite permissions)
pub async fn create_law(
    _user: crate::middleware::AuthenticatedUser,
    Json(request): Json<CreateLawRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let law_id = Uuid::new_v4().to_string();
    
    Ok((StatusCode::CREATED, Json(json!({
        "id": law_id,
        "title": request.title,
        "description": request.description,
        "content": request.content,
        "category": request.category,
        "status": "draft",
        "created_at": chrono::Utc::now().to_rfc3339()
    }))))
}

/// Modifier une loi existante
pub async fn update_law(
    Path(id): Path<String>,
    _user: crate::middleware::AuthenticatedUser,
    Json(request): Json<CreateLawRequest>,
) -> Json<serde_json::Value> {
    Json(json!({
        "id": id,
        "title": request.title,
        "description": request.description,
        "content": request.content,
        "category": request.category,
        "status": "modified",
        "updated_at": chrono::Utc::now().to_rfc3339()
    }))
}

/// Supprimer une loi
pub async fn delete_law(
    Path(id): Path<String>,
    _user: crate::middleware::AuthenticatedUser,
) -> StatusCode {
    // Simuler la suppression
    StatusCode::NO_CONTENT
}

// =============================================================================
// HANDLERS DE VOTE
// =============================================================================

/// Voter sur une loi
pub async fn vote_on_law(
    _user: crate::middleware::AuthenticatedUser,
    Json(request): Json<VoteRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(json!({
        "law_id": request.law_id,
        "vote": request.vote_type,
        "comment": request.comment,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "status": "recorded"
    })))
}

// =============================================================================
// HANDLERS D'ADMINISTRATION  
// =============================================================================

/// Lister tous les utilisateurs (Admin seulement)
pub async fn list_users(
    _user: crate::middleware::AuthenticatedUser,
) -> Json<Vec<serde_json::Value>> {
    Json(vec![
        json!({
            "id": "user-1",
            "name": "Marie Dupont", 
            "email": "marie@example.com",
            "role": "Citizen",
            "reputation": 100,
            "active": true
        }),
        json!({
            "id": "user-2",
            "name": "Jean Martin",
            "email": "jean@example.com", 
            "role": "Representative",
            "reputation": 150,
            "active": true
        })
    ])
}

/// Statistiques du système (Auditeur+)
pub async fn get_statistics(
    _user: crate::middleware::AuthenticatedUser,
) -> Json<serde_json::Value> {
    Json(json!({
        "total_laws": 42,
        "active_laws": 38,
        "total_votes": 1247,
        "active_users": 156,
        "system_health": "excellent"
    }))
}

// =============================================================================
// CONFIGURATION DU ROUTEUR
// =============================================================================

pub fn create_router_with_state(app_state: AppState) -> Router {
    // Routes publiques (sans authentification)
    let public_routes = Router::new()
        .route("/auth/login", post(login))
        .route("/auth/register", post(register));
    
    // Routes protégées par authentification de base
    let protected_routes = Router::new()
        .route("/auth/profile", get(profile))
        .route("/auth/logout", post(logout)) 
        .route("/laws", get(list_laws))
        .route("/laws/:id", get(get_law))
        .route_layer(middleware::from_fn_with_state(
            app_state.auth_state.clone(),
            auth_middleware,
        ));
    
    // Routes nécessitant des permissions spéciales
    let admin_routes = Router::new()
        .route("/laws", post(create_law))
        .route("/laws/:id", put(update_law))
        .route("/laws/:id", delete(delete_law))
        .route("/vote", post(vote_on_law))
        .route("/admin/users", get(list_users))
        .route("/stats", get(get_statistics))
        .route_layer(middleware::from_fn_with_state(
            app_state.auth_state.clone(),
            auth_middleware,
        ));
    
    // Combiner toutes les routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(app_state)
}

/// Helper pour les tests - crée un router avec un AuthManager
pub fn create_router(auth_manager: AuthManager) -> Router {
    let app_state = AppState::new(auth_manager);
    
    Router::new()
        // Route de santé publique
        .route("/health", axum::routing::get(|| async { 
            axum::Json(serde_json::json!({
                "status": "healthy",
                "service": "api-gateway",
                "version": "1.0.0"
            }))
        }))
        // Routes API avec préfixe /api
        .nest("/api", create_router_with_state(app_state))
}