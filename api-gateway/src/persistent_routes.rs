use axum::{
    routing::{get, post},
    Router,
    Json,
    Extension,
    http::{StatusCode, HeaderMap},
    extract::{Path, Query},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;
use anyhow::anyhow;
use sha2::Digest;
use base64::Engine;

use crate::handler_utils::{into_response, success_response, error_response, simple_error, handle_result};

use crate::models::*;
use crate::user_management::UserManagementService;
use crate::middleware::AuthenticatedUser;
use crate::models::{UserProfile, IdentityValidationStatus, CryptoKeysInfo};
use crate::persistent_database::DatabaseService;

/// Routes pour le système d'utilisateurs persistant
pub fn create_persistent_routes() -> Router {
    Router::new()
        // Authentification
        .route("/api/v2/auth/register", post(register_handler))
        .route("/api/v2/auth/login", post(login_handler))
        .route("/api/v2/auth/logout", post(logout_handler))
        .route("/api/v2/auth/profile", get(profile_handler))
        
        // Validation d'identité
        .route("/api/v2/identity/submit", post(submit_identity_handler))
        .route("/api/v2/identity/status", get(identity_status_handler))
        .route("/api/v2/identity/callback/:transaction_id", post(identity_callback_handler))
        
        // Gestion des clés cryptographiques
        .route("/api/v2/crypto/generate", post(generate_keys_handler))
        .route("/api/v2/crypto/info", get(crypto_keys_info_handler))
        .route("/api/v2/crypto/sign", post(sign_message_handler))
        .route("/api/v2/crypto/verify", post(verify_signature_handler))
        .route("/api/v2/crypto/backup", post(backup_keys_handler))
        .route("/api/v2/crypto/restore", post(restore_keys_handler))
        
        // Vote (uniquement pour utilisateurs validés avec clés)
        .route("/api/v2/vote/eligibility", get(vote_eligibility_handler))
        .route("/api/v2/vote/cast", post(cast_vote_handler))
        
        // Administration
        .route("/api/v2/admin/users", get(admin_list_users_handler))
        .route("/api/v2/admin/identity/:user_id/approve", post(admin_approve_identity_handler))
        .route("/api/v2/admin/identity/:user_id/reject", post(admin_reject_identity_handler))
}

// ========== STRUCTURES DE REQUÊTE/RÉPONSE ==========

#[derive(Debug, Deserialize)]
struct GenerateKeysRequest {
    password: String,
}

#[derive(Debug, Deserialize)]
struct SignMessageRequest {
    password: String,
    message: String, // Base64 ou hex
}

#[derive(Debug, Deserialize)]
struct VerifySignatureRequest {
    user_id: Uuid,
    message: String, // Base64 ou hex  
    signature: String, // Base64 ou hex
}

#[derive(Debug, Deserialize)]
struct BackupKeysRequest {
    password: String,
}

#[derive(Debug, Deserialize)]
struct RestoreKeysRequest {
    password: String,
    backup_data: String,
}

#[derive(Debug, Deserialize)]
struct CastVoteRequest {
    password: String,
    law_id: Uuid,
    vote: String, // "pour", "contre", "abstention"
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminRejectRequest {
    reason: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> Default for ApiResponse<T> where T: Default {
    fn default() -> Self {
        Self {
            success: true,
            data: Some(T::default()),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self where T: Default {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error_empty(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error_for<U>(message: impl Into<String>) -> ApiResponse<U> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl<T> IntoResponse for ApiResponse<T> 
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        let status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        
        (status, Json(self)).into_response()
    }
}

// ========== HANDLERS D'AUTHENTIFICATION ==========

async fn register_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    Json(request): Json<RegisterRequest>
) -> impl IntoResponse {
    match user_service.register_user(&request).await {
        Ok(user) => into_response(StatusCode::CREATED, ApiResponse::success(user)),
        Err(e) => error_response::<UserProfile>(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn login_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>
) -> impl IntoResponse {
    let ip_address = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or_default().to_string();
    
    let user_agent = headers.get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default().to_string();

    match user_service.login_user(&request, ip_address, user_agent).await {
        Ok(response) => success_response(response),
        Err(e) => error_response::<LoginResponse>(StatusCode::UNAUTHORIZED, &e.to_string()),
    }
}

async fn logout_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if let Some(token) = token {
        match user_service.logout_user(token).await {
            Ok(_) => success_response("Déconnexion réussie"),
            Err(e) => error_response::<String>(StatusCode::BAD_REQUEST, &e.to_string()),
        }
    } else {
        error_response::<String>(StatusCode::BAD_REQUEST, "Token manquant")
    }
}

async fn profile_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Essayer d'extraire le token de type "Bearer <token>"
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    // Si pas de token, refuser l'accès
    let Some(token) = token else {
        return error_response::<UserProfile>(StatusCode::UNAUTHORIZED, "Token manquant ou invalide");
    };

    // Décoder le token simple (base64 JSON) émis par UserManagementService::generate_jwt_token
    #[derive(serde::Deserialize)]
    struct SimpleTokenPayload {
        #[serde(rename = "user_id")] 
        user_id: String,
        // champs additionnels ignorés
    }

    let payload_json = match base64::engine::general_purpose::STANDARD.decode(token) {
        Ok(bytes) => bytes,
        Err(_) => return error_response::<UserProfile>(StatusCode::UNAUTHORIZED, "Token illisible"),
    };

    let payload: SimpleTokenPayload = match serde_json::from_slice(&payload_json) {
        Ok(p) => p,
        Err(_) => return error_response::<UserProfile>(StatusCode::UNAUTHORIZED, "Token invalide"),
    };

    let user_id = match uuid::Uuid::parse_str(&payload.user_id) {
        Ok(id) => id,
        Err(_) => return error_response::<UserProfile>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };

    match user_service.get_user_profile(user_id).await {
        Ok(profile) => success_response(profile),
        Err(e) => error_response::<UserProfile>(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// ========== HANDLERS DE VALIDATION D'IDENTITÉ ==========

async fn submit_identity_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Json(request): Json<SubmitIdentityRequest>
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<IdentityValidationStatus>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };
    
    match user_service.submit_identity_document(&user_id, &request).await {
        Ok(validation) => success_response(validation),
        Err(e) => error_response::<IdentityValidationStatus>(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn identity_status_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<IdentityValidationStatus>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };
    
    match user_service.get_identity_validation_status(user_id).await {
        Ok(Some(status)) => success_response(status),
        Ok(None) => error_response::<IdentityValidationStatus>(StatusCode::NOT_FOUND, "Aucune demande de validation trouvée"),
        Err(e) => error_response::<IdentityValidationStatus>(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn identity_callback_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    Path(transaction_id): Path<String>,
    Json(callback_data): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Traitement des callbacks de l'API gouvernementale
    tracing::info!("Callback reçu pour transaction {}: {:?}", transaction_id, callback_data);
    
    // TODO: Implémenter le traitement du callback
    (StatusCode::OK, Json(ApiResponse::success("Callback traité")))
}

// ========== HANDLERS DE GESTION DES CLÉS ==========

async fn generate_keys_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Json(request): Json<GenerateKeysRequest>
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<CryptoKeysInfo>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };
    
    match user_service.generate_user_crypto_keys(&user_id, &request.password).await {
        Ok(keys_info) => into_response(StatusCode::CREATED, ApiResponse::success(keys_info)),
        Err(e) => error_response::<CryptoKeysInfo>(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn crypto_keys_info_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<CryptoKeysInfo>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };
    
    match user_service.get_user_crypto_keys_info(&user_id).await {
        Ok(keys_info) => success_response(keys_info),
        Err(e) => error_response::<CryptoKeysInfo>(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn sign_message_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Json(request): Json<SignMessageRequest>
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<serde_json::Value>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };

    let message_bytes = match hex::decode(&request.message) {
        Ok(bytes) => bytes,
        Err(_) => match base64::engine::general_purpose::STANDARD.decode(&request.message) {
            Ok(bytes) => bytes,
            Err(_) => request.message.as_bytes().to_vec(),
        }
    };

    match user_service.sign_message(&user_id, &request.password, &message_bytes).await {
        Ok(signature) => {
            let signature_response = serde_json::json!({
                "signature_hex": hex::encode(&signature),
                "signature_base64": base64::engine::general_purpose::STANDARD.encode(&signature),
                "message_hash": hex::encode(sha2::Sha256::digest(&message_bytes))
            });
            success_response(signature_response)
        },
        Err(e) => error_response::<serde_json::Value>(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn verify_signature_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    Json(request): Json<VerifySignatureRequest>
) -> impl IntoResponse {
    let message_bytes = match hex::decode(&request.message) {
        Ok(bytes) => bytes,
        Err(_) => match base64::engine::general_purpose::STANDARD.decode(&request.message) {
            Ok(bytes) => bytes,
            Err(_) => request.message.as_bytes().to_vec(),
        }
    };

    let signature_bytes = match hex::decode(&request.signature) {
        Ok(bytes) => bytes,
        Err(_) => match base64::engine::general_purpose::STANDARD.decode(&request.signature) {
            Ok(bytes) => bytes,
            Err(_) => return error_response::<serde_json::Value>(StatusCode::BAD_REQUEST, "Format de signature invalide")
        }
    };

    match user_service.verify_signature(&request.user_id, &message_bytes, &signature_bytes).await {
        Ok(is_valid) => {
            let verification_response = serde_json::json!({
                "valid": is_valid,
                "user_id": request.user_id,
                "message_hash": hex::encode(sha2::Sha256::digest(&message_bytes))
            });
            success_response(verification_response)
        },
        Err(e) => error_response::<serde_json::Value>(StatusCode::BAD_REQUEST, &e.to_string()),
    }
}

async fn backup_keys_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Json(request): Json<BackupKeysRequest>
) -> impl IntoResponse {
    // TODO: Implémenter le backup des clés
    error_response::<serde_json::Value>(StatusCode::NOT_IMPLEMENTED, "Backup non encore implémenté")
}

async fn restore_keys_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Json(request): Json<RestoreKeysRequest>
) -> impl IntoResponse {
    // TODO: Implémenter la restauration des clés
    error_response::<serde_json::Value>(StatusCode::NOT_IMPLEMENTED, "Restauration non encore implémentée")
}

// ========== HANDLERS DE VOTE ==========

async fn vote_eligibility_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<serde_json::Value>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };
    
    match user_service.can_user_vote(user_id).await {
        Ok(can_vote) => {
            let eligibility = serde_json::json!({
                "can_vote": can_vote,
                "user_id": user_id,
                "reasons": if !can_vote {
                    vec!["Identité non validée ou clés cryptographiques manquantes"]
                } else {
                    vec![]
                }
            });
            success_response(eligibility)
        },
        Err(e) => error_response::<serde_json::Value>(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

async fn cast_vote_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Json(request): Json<CastVoteRequest>
) -> impl IntoResponse {
    let user_id = match user.claims.user_id() {
        Ok(id) => id,
        Err(_) => return error_response::<serde_json::Value>(StatusCode::BAD_REQUEST, "ID utilisateur invalide"),
    };
    
    // Vérifier l'éligibilité
    match user_service.can_user_vote(user_id).await {
        Ok(false) => return error_response::<serde_json::Value>(StatusCode::FORBIDDEN, "Vous n'êtes pas autorisé à voter"),
        Err(e) => return error_response::<serde_json::Value>(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        Ok(true) => {}
    }

    // TODO: Implémenter le vote avec signature cryptographique
    let vote_data = serde_json::json!({
        "law_id": request.law_id,
        "vote": request.vote,
        "comment": request.comment,
        "user_id": user_id,
        "timestamp": chrono::Utc::now()
    });

    // TODO: Signer le vote avec les clés de l'utilisateur
    // TODO: Enregistrer le vote dans la blockchain

    into_response(StatusCode::CREATED, ApiResponse::<serde_json::Value>::success(vote_data))
}

// ========== HANDLERS D'ADMINISTRATION ==========

async fn admin_list_users_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
) -> impl IntoResponse {
    // Vérifier les droits administrateur
    if !matches!(user.claims.role, common::UserRole::Administrator) {
        return error_response::<serde_json::Value>(StatusCode::FORBIDDEN, "Accès administrateur requis");
    }

    // TODO: Implémenter la liste des utilisateurs pour admin
    error_response::<serde_json::Value>(StatusCode::NOT_IMPLEMENTED, "Liste administrateur non implémentée")
}

async fn admin_approve_identity_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    // Vérifier les droits administrateur
    if !matches!(user.claims.role, common::UserRole::Administrator) {
        return error_response::<serde_json::Value>(StatusCode::FORBIDDEN, "Accès administrateur requis");
    }

    // TODO: Implémenter l'approbation manuelle d'identité
    error_response::<serde_json::Value>(StatusCode::NOT_IMPLEMENTED, "Approbation manuelle non implémentée")
}

async fn admin_reject_identity_handler(
    Extension(user_service): Extension<Arc<UserManagementService>>,
    user: AuthenticatedUser,
    Path(user_id): Path<Uuid>,
    Json(request): Json<AdminRejectRequest>,
) -> impl IntoResponse {
    // Vérifier les droits administrateur
    if !matches!(user.claims.role, common::UserRole::Administrator) {
        return error_response::<serde_json::Value>(StatusCode::FORBIDDEN, "Accès administrateur requis");
    }

    // TODO: Implémenter le rejet manuel d'identité
    error_response::<serde_json::Value>(StatusCode::NOT_IMPLEMENTED, "Rejet manuel non implémenté")
}
