use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use crate::persistent_routes::ApiResponse;

/// Fonction utilitaire pour convertir les types de réponse communs en Response pour Axum
pub fn into_response<T: Serialize>(status: StatusCode, body: ApiResponse<T>) -> Response {
    (status, Json(body)).into_response()
}

/// Fonction utilitaire pour les réponses réussies
pub fn success_response<T: Serialize>(data: T) -> Response {
    into_response(StatusCode::OK, ApiResponse::success(data))
}

/// Fonction utilitaire pour les réponses d'erreur
pub fn error_response<T: Serialize>(status: StatusCode, message: &str) -> Response {
    into_response(status, ApiResponse::<T>::error_empty(message.to_string()))
}

/// Fonction utilitaire pour les réponses d'erreur sans type générique
pub fn simple_error(status: StatusCode, message: &str) -> Response {
    into_response(status, ApiResponse::<()>::error_empty(message.to_string()))
}

/// Fonction utilitaire pour transformer un Result en Response
pub fn handle_result<T, E>(result: Result<T, E>, success_status: StatusCode, error_status: StatusCode) -> Response
where
    T: Serialize,
    E: std::fmt::Display,
{
    match result {
        Ok(data) => into_response(success_status, ApiResponse::success(data)),
        Err(e) => into_response(error_status, ApiResponse::<T>::error_empty(e.to_string())),
    }
}