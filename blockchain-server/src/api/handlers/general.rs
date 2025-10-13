use axum::{extract::State, response::Json};
use serde_json;
use super::super::AppState;

/// Handler racine
pub async fn root_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "E-Government Blockchain API",
        "version": "0.1.0",
        "description": "API for e-government blockchain system",
        "endpoints": {
            "health": "/health",
            "stats": "/stats",
            "info": "/info",
            "api": "/api/*",
            "rpc": "/rpc/*"
        }
    }))
}

/// Handler de santé
pub async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handler des statistiques
pub async fn stats_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let stats = node.get_stats().await;
    Json(serde_json::to_value(stats).unwrap())
}

/// Handler des informations blockchain
pub async fn blockchain_info_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let info = node.get_blockchain_info().await;
    Json(serde_json::to_value(info).unwrap())
}