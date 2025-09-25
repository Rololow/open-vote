use axum::{extract::State, response::Json, http::StatusCode};
use serde_json;
use tracing::{info, error};
use super::super::{AppState, types::*};

/// Handler pour diffuser une transaction signée au réseau
pub async fn broadcast_transaction_handler(
    State(node): State<AppState>,
    Json(request): Json<BroadcastTxRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tx = request.transaction;

    // 1. Verify transaction signature
    if let Err(e) = tx.verify() {
        error!("Transaction signature verification failed for transaction {}: {:?}", tx.id, e);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid_signature".into(),
                message: "Transaction signature is invalid".into(),
            }),
        ));
    }

    // 2. Submit transaction to node mempool
    match node.submit_transaction(tx.clone()).await {
        Ok(_) => {
            info!("Transaction {} received via RPC and added to mempool", tx.id);
            // Peer propagation is handled by submit_transaction
            Ok(Json(serde_json::json!({
                "status": "pending",
                "message": "Transaction received and added to mempool",
                "transaction_id": tx.id.to_string()
            })))
        }
        Err(e) => {
            error!("Error submitting transaction {} via RPC: {:?}", tx.id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "submission_failed".into(),
                    message: format!("Failed to submit transaction: {}", e),
                }),
            ))
        }
    }
}