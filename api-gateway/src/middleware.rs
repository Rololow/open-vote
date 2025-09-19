use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::{AuthManager, AuthError, JwtClaims, UserRole};

/// État partagé pour l'authentification
#[derive(Clone)]
pub struct AuthState {
    pub auth_manager: Arc<RwLock<AuthManager>>,
}

/// Extension pour ajouter les claims d'un utilisateur authentifié à la requête
#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub claims: JwtClaims,
}

/// Erreur d'authentification pour les réponses HTTP
#[derive(Debug)]
pub struct AuthenticationError {
    pub message: String,
    pub status_code: StatusCode,
}

impl IntoResponse for AuthenticationError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": "Authentication failed",
            "message": self.message,
            "status": self.status_code.as_u16()
        }));
        
        (self.status_code, body).into_response()
    }
}

impl From<AuthError> for AuthenticationError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::TokenExpired => AuthenticationError {
                message: "Token has expired".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            },
            AuthError::InvalidToken(msg) => AuthenticationError {
                message: format!("Invalid token: {}", msg),
                status_code: StatusCode::UNAUTHORIZED,
            },
            AuthError::InvalidSignature => AuthenticationError {
                message: "Invalid token signature".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            },
            AuthError::InsufficientPermissions => AuthenticationError {
                message: "Insufficient permissions".to_string(),
                status_code: StatusCode::FORBIDDEN,
            },
            AuthError::UserNotFound(id) => AuthenticationError {
                message: format!("User not found: {}", id),
                status_code: StatusCode::NOT_FOUND,
            },
            AuthError::SerializationError(msg) => AuthenticationError {
                message: format!("Serialization error: {}", msg),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}

/// Middleware d'authentification qui vérifie le token JWT
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthenticationError> {
    // Extraire le header Authorization
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| AuthenticationError {
            message: "Missing Authorization header".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        })?;

    // Vérifier le format Bearer
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AuthenticationError {
            message: "Invalid Authorization header format. Expected 'Bearer <token>'".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        })?;

    // Valider le token
    let auth_manager = auth_state.auth_manager.read().await;
    let claims = auth_manager.validate_token(token)
        .map_err(AuthenticationError::from)?;

    // Ajouter les claims à la requête
    request.extensions_mut().insert(AuthenticatedUser { claims });

    // Continuer vers le handler suivant
    Ok(next.run(request).await)
}

/// Middleware pour vérifier qu'un utilisateur a un rôle spécifique ou supérieur
pub fn require_role(required_role: UserRole) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AuthenticationError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_role = required_role.clone();
        Box::pin(async move {
            // Récupérer l'utilisateur authentifié
            let authenticated_user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or_else(|| AuthenticationError {
                    message: "User not authenticated".to_string(),
                    status_code: StatusCode::UNAUTHORIZED,
                })?;

            // Vérifier le rôle
            if !authenticated_user.claims.role.has_permission(&required_role) {
                return Err(AuthenticationError {
                    message: format!(
                        "Insufficient role. Required: {:?}, Current: {:?}",
                        required_role, authenticated_user.claims.role
                    ),
                    status_code: StatusCode::FORBIDDEN,
                });
            }

            Ok(next.run(request).await)
        })
    }
}

/// Middleware pour vérifier qu'un utilisateur a une permission spécifique
pub fn require_permission(permission: &'static str) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AuthenticationError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        Box::pin(async move {
            // Récupérer l'utilisateur authentifié
            let authenticated_user = request
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or_else(|| AuthenticationError {
                    message: "User not authenticated".to_string(),
                    status_code: StatusCode::UNAUTHORIZED,
                })?;

            // Vérifier la permission
            if !authenticated_user.claims.has_permission(permission) {
                return Err(AuthenticationError {
                    message: format!(
                        "Missing required permission: {}",
                        permission
                    ),
                    status_code: StatusCode::FORBIDDEN,
                });
            }

            Ok(next.run(request).await)
        })
    }
}

/// Extracteur pour récupérer facilement l'utilisateur authentifié dans les handlers
#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthenticationError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| AuthenticationError {
                message: "User not authenticated".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })
    }
}

/// Macro pour faciliter la création de middlewares de permission
#[macro_export]
macro_rules! require_permission {
    ($permission:expr) => {
        axum::middleware::from_fn(crate::middleware::require_permission($permission))
    };
}

/// Macro pour faciliter la création de middlewares de rôle
#[macro_export]
macro_rules! require_role {
    ($role:expr) => {
        axum::middleware::from_fn(crate::middleware::require_role($role))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{JwtClaims, UserRole, permissions};
    use axum::{body::Body, http::Request};
    use crypto_lib::KeyPair;
    use uuid::Uuid;

    fn create_test_claims(role: UserRole, permissions: Vec<String>) -> JwtClaims {
        let user_id = Uuid::new_v4();
        let public_key = KeyPair::generate().public_key().clone();
        
        JwtClaims::new(user_id, public_key, role, permissions, 24)
    }

    #[test]
    fn test_auth_error_to_http_error() {
        let token_error = AuthError::InvalidToken("test error".to_string());
        let http_error = AuthenticationError::from(token_error);
        
        assert_eq!(http_error.status_code, StatusCode::UNAUTHORIZED);
        assert!(http_error.message.contains("test error"));
    }

    #[test]
    fn test_expired_token_error() {
        let expired_error = AuthError::TokenExpired;
        let http_error = AuthenticationError::from(expired_error);
        
        assert_eq!(http_error.status_code, StatusCode::UNAUTHORIZED);
        assert_eq!(http_error.message, "Token has expired");
    }

    #[test]
    fn test_insufficient_permissions_error() {
        let perm_error = AuthError::InsufficientPermissions;
        let http_error = AuthenticationError::from(perm_error);
        
        assert_eq!(http_error.status_code, StatusCode::FORBIDDEN);
        assert_eq!(http_error.message, "Insufficient permissions");
    }

    #[test]
    fn test_authenticated_user_creation() {
        let claims = create_test_claims(
            UserRole::Citizen,
            vec![permissions::VOTE_ON_LAW.to_string()]
        );
        
        let auth_user = AuthenticatedUser { claims: claims.clone() };
        
        assert_eq!(auth_user.claims.role, UserRole::Citizen);
        assert!(auth_user.claims.has_permission(permissions::VOTE_ON_LAW));
    }
}