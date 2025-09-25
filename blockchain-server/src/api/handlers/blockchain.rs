use axum::{extract::{State, Path, Query}, response::Json, http::StatusCode};
use serde_json;
use tracing::{info, error};
use super::super::{AppState, types::*};
use uuid::Uuid;
use chrono::Utc;

/// Handler pour obtenir les blocs
pub async fn get_blocks_handler(
    #[allow(unused_variables)]
    State(node): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let blockchain = node.blockchain.read().await;
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10).min(100); // Max 100 per page
    
    let total_blocks = blockchain.blocks.len();
    let start_idx = ((page - 1) * limit) as usize;
    let end_idx = (start_idx + limit as usize).min(total_blocks);
    
    let blocks: Vec<_> = blockchain.blocks[start_idx..end_idx]
        .iter()
        .map(|block| serde_json::json!({
            "number": block.header.block_number,
            "hash": block.hash.to_hex(),
            "previous_hash": block.header.previous_hash.to_hex(),
            "timestamp": block.header.timestamp,
            "transaction_count": block.transactions.len()
        }))
        .collect();
    
    Ok(Json(serde_json::json!({
        "blocks": blocks,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total_blocks,
            "total_pages": (total_blocks as f64 / limit as f64).ceil() as u32
        }
    })))
}

/// Handler pour obtenir un bloc spécifique
pub async fn get_block_handler(
    #[allow(unused_variables)]
    State(node): State<AppState>,
    Path(block_number): Path<u64>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let blockchain = node.blockchain.read().await;
    
    if let Some(block) = blockchain.blocks.get(block_number as usize) {
        Ok(Json(serde_json::to_value(block).unwrap()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Handler pour obtenir le dernier bloc
pub async fn get_latest_block_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    let latest_block = blockchain.last_block();
    Json(serde_json::to_value(latest_block).unwrap())
}

/// Handler pour miner un bloc
pub async fn mine_block_handler(State(node): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Manual mining request received");
    
    match node.mine_block().await {
        Ok(Some(block)) => {
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "Block mined successfully",
                "block": {
                    "number": block.header.block_number,
                    "hash": block.hash.to_hex(),
                    "transactions": block.transactions.len()
                }
            })))
        }
        Ok(None) => {
            Ok(Json(serde_json::json!({
                "status": "info",
                "message": "No transactions to mine"
            })))
        }
        Err(e) => {
            error!("Mining error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handler pour soumettre une transaction
pub async fn submit_transaction_handler(
    #[allow(unused_variables)]
    State(node): State<AppState>,
    Json(request): Json<SubmitTransactionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement transaction creation and validation
    // For now, return a placeholder
    
    info!("Transaction received: {}", request.transaction_type);
    
    Ok(Json(serde_json::json!({
        "status": "pending",
        "message": "Transaction added to mempool",
        "transaction_id": uuid::Uuid::new_v4().to_string()
    })))
}

// ==============================
// Proposals (propositions de loi)
// ==============================

/// Crée une nouvelle proposition citoyenne en mémoire
pub async fn create_proposal_handler(
    State(node): State<AppState>,
    Json(req): Json<CreateProposalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Validation minimale
    if req.title.trim().is_empty() || req.description.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "invalid_input".into(), message: "title et description sont requis".into() })
        ));
    }

    let proposal = crate::node::Proposal {
        id: Uuid::new_v4(),
        title: req.title.trim().to_string(),
        category: req.category.trim().to_string(),
        description: req.description.trim().to_string(),
        full_text: req.full_text.unwrap_or_default(),
        estimated_budget: req.estimated_budget,
        implementation_timeline: req.implementation_timeline,
        tags: req.tags.unwrap_or_default(),
        author_id: req.author_id,
        author_name: req.author_name,
        created_at: Utc::now(),
        status: "Collecte signatures".into(),
        supporters: 0,
        upvotes: 0,
        downvotes: 0,
        comments: Vec::new(),
        signatures_required: 100, // valeur arbitraire de démo
    };

    node.add_proposal(proposal.clone()).await;
    info!("📜 Nouvelle proposition créée: {}", proposal.id);

    Ok(Json(serde_json::json!({
        "status": "created",
        "proposal_id": proposal.id.to_string(),
        "message": "Proposition enregistrée (en mémoire)",
    })))
}

/// Liste les propositions
pub async fn list_proposals_handler(
    State(node): State<AppState>,
) -> Json<serde_json::Value> {
    let proposals = node.list_proposals().await;
    Json(serde_json::json!({
        "proposals": proposals
    }))
}

/// Récupère une proposition par ID
pub async fn get_proposal_handler(
    State(node): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Ok(uuid) = Uuid::parse_str(&id) else { return Err(StatusCode::BAD_REQUEST); };
    if let Some(p) = node.get_proposal(&uuid).await {
        Ok(Json(serde_json::json!(p)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Ajoute un soutien à une proposition
pub async fn support_proposal_handler(
    State(node): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Ok(uuid) = Uuid::parse_str(&id) else { return Err(StatusCode::BAD_REQUEST); };
    if node.support_proposal(&uuid).await {
        Ok(Json(serde_json::json!({
            "status": "ok",
            "proposal_id": id,
            "message": "Support ajouté"
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}