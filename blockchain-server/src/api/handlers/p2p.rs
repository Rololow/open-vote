use axum::{extract::State, response::Json, http::StatusCode};
use serde_json;
use common::{Block, Transaction};
use super::super::{AppState, types::*};

/// Peers: list
pub async fn get_peers_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let peers = node.get_peers().await;
    Json(serde_json::json!({ "peers": peers }))
}

/// Peers extended (with last_seen)
pub async fn get_peers_extended_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let peers = node.get_peers_extended().await;
    Json(serde_json::json!({ "peers": peers }))
}

/// Peers: add
pub async fn add_peer_handler(
    State(node): State<AppState>,
    Json(req): Json<AddPeerRequest>,
) -> Json<serde_json::Value> {
    node.add_peer(req.address.clone()).await;
    Json(serde_json::json!({ "status": "ok" }))
}

/// Discovery: accept a list of peers, merge them, return our known peers
pub async fn peers_discovery_handler(
    State(node): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(list) = req.get("peers").and_then(|v| v.as_array()) {
        for p in list { if let Some(addr) = p.as_str() { node.add_peer(addr.to_string()).await; } }
    }
    let peers = node.get_peers().await;
    Json(serde_json::json!({ "peers": peers }))
}

/// Heartbeat: marks caller as alive (address provided) and returns ok
pub async fn peers_heartbeat_handler(
    State(node): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(addr) = req.get("address").and_then(|v| v.as_str()) { node.add_peer(addr.to_string()).await; }
    Json(serde_json::json!({ "status": "pong" }))
}

/// Receive a block from network
pub async fn receive_block_handler(
    State(node): State<AppState>,
    Json(block): Json<Block>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match node.receive_block(block).await {
        Ok(true) => Ok(Json(serde_json::json!({ "status": "accepted" }))),
        Ok(false) => Ok(Json(serde_json::json!({ "status": "rejected" }))),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Receive a transaction from network
pub async fn receive_tx_handler(
    State(node): State<AppState>,
    Json(tx): Json<Transaction>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Verify and add to mempool
    if let Err(_) = tx.verify() { 
        return Err(StatusCode::BAD_REQUEST); 
    }
    if let Err(_) = node.submit_transaction(tx).await { 
        return Err(StatusCode::BAD_REQUEST); 
    }
    Ok(Json(serde_json::json!({ "status": "received" })))
}