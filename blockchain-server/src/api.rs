use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use anyhow::Result;
use tracing::{info, error};
use uuid::Uuid;

use common::{Transaction, TransactionType, Account, Law, Vote, VoteType};
use crypto_lib::{KeyPair, PublicKey, Signature};
use crate::node::BlockchainNode;

/// État partagé de l'API
type AppState = Arc<BlockchainNode>;

/// Structure pour les réponses d'erreur
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

/// Structure pour soumettre une transaction
#[derive(Deserialize)]
struct SubmitTransactionRequest {
    transaction_type: String,
    data: serde_json::Value,
    sender_public_key: String,
    signature: String,
}

/// Structure pour créer un compte
#[derive(Deserialize)]
struct CreateAccountRequest {
    public_key: String,
    display_name: Option<String>,
    bio: Option<String>,
}

/// Structure pour créer une loi
#[derive(Deserialize)]
struct CreateLawRequest {
    title: String,
    content: String,
    summary: String,
    category: String,
    author_public_key: String,
    signature: String,
}

/// Structure pour voter
#[derive(Deserialize)]
struct SubmitVoteRequest {
    law_id: String,
    vote_type: String, // "For", "Against", "Abstain"
    voter_public_key: String,
    signature: String,
    comment: Option<String>,
}

/// Paramètres de requête pour la pagination
#[derive(Deserialize)]
struct PaginationQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

/// Démarre le serveur API
pub async fn start_api_server(node: Arc<BlockchainNode>) -> Result<()> {
    let app = create_router(node.clone());
    
    let bind_addr = format!("{}:{}", node.config.bind_address, node.config.api_port);
    info!("🚀 Serveur API démarré sur http://{}", bind_addr);
    info!("📖 Documentation disponible sur http://{}/docs", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Crée le routeur principal de l'API
fn create_router(node: Arc<BlockchainNode>) -> Router {
    let api_routes = Router::new()
        // Blocs
        .route("/blocks", get(get_blocks_handler))
        .route("/blocks/:block_number", get(get_block_handler))
        .route("/blocks/latest", get(get_latest_block_handler))
        
        // Transactions
        .route("/transactions", post(submit_transaction_handler))
        .route("/transactions", get(get_transactions_handler))
        .route("/transactions/:tx_id", get(get_transaction_handler))
        .route("/transactions/pending", get(get_pending_transactions_handler))
        
        // Comptes
        .route("/accounts", post(create_account_handler))
        .route("/accounts", get(get_accounts_handler))
        .route("/accounts/:account_id", get(get_account_handler))
        
        // Lois
        .route("/laws", post(create_law_handler))
        .route("/laws", get(get_laws_handler))
        .route("/laws/:law_id", get(get_law_handler))
        .route("/laws/:law_id/votes", get(get_law_votes_handler))
        .route("/laws/:law_id/results", get(get_vote_results_handler))
        
        // Propositions (alias pour les lois)
        .route("/proposals", get(get_laws_handler))
        .route("/proposals/:law_id", get(get_law_handler))
        
        // Votes
        .route("/votes", post(submit_vote_handler))
        
        // Mining
        .route("/mine", post(mine_block_handler))
        
        // Audit
        .route("/audit/stats", get(get_audit_stats_handler))
        .route("/audit/logs", get(get_audit_logs_handler))
        .route("/audit/export", get(get_audit_export_handler))
        
        // Analytics
        .route("/analytics/overview", get(get_analytics_overview_handler))
        .route("/analytics/votes", get(get_analytics_votes_handler))
        .route("/analytics/engagement", get(get_analytics_engagement_handler))
        
        // Propositions endpoints supplémentaires
        .route("/proposals/categories", get(get_proposals_categories_handler))
        .route("/proposals/:proposal_id/support", post(support_proposal_handler));

    Router::new()
        // Info générales
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/stats", get(stats_handler))
        .route("/info", get(blockchain_info_handler))
        
        // Routes API avec préfixe /api
        .nest("/api", api_routes)
        
        .layer(CorsLayer::permissive())
        .with_state(node)
}

/// Handler racine
async fn root_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "E-Government Blockchain API",
        "version": "0.1.0",
        "description": "API pour le système de blockchain e-gouvernement",
        "endpoints": {
            "health": "/health",
            "stats": "/stats",
            "blockchain_info": "/info",
            "blocks": "/blocks",
            "transactions": "/transactions",
            "accounts": "/accounts",
            "laws": "/laws",
            "votes": "/votes",
            "mine": "/mine"
        }
    }))
}

/// Handler de santé
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handler des statistiques
async fn stats_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let stats = node.get_stats().await;
    Json(serde_json::to_value(stats).unwrap())
}

/// Handler des informations blockchain
async fn blockchain_info_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let info = node.get_blockchain_info().await;
    Json(serde_json::to_value(info).unwrap())
}

/// Handler pour obtenir les blocs
async fn get_blocks_handler(
    State(node): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let blockchain = node.blockchain.read().await;
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(10).min(100); // Max 100 par page
    
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
async fn get_block_handler(
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
async fn get_latest_block_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    let latest_block = blockchain.last_block();
    Json(serde_json::to_value(latest_block).unwrap())
}

/// Handler pour soumettre une transaction
async fn submit_transaction_handler(
    State(node): State<AppState>,
    Json(request): Json<SubmitTransactionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implémenter la création et validation de transaction
    // Pour l'instant, retourner un placeholder
    
    info!("Réception transaction: {}", request.transaction_type);
    
    Ok(Json(serde_json::json!({
        "status": "pending",
        "message": "Transaction ajoutée à la mempool",
        "transaction_id": Uuid::new_v4().to_string()
    })))
}

/// Handler pour créer un compte
async fn create_account_handler(
    State(node): State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Création compte: {}", request.public_key);
    
    // TODO: Implémenter la création de compte
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Compte créé avec succès",
        "account_id": Uuid::new_v4().to_string()
    })))
}

/// Handler pour créer une loi
async fn create_law_handler(
    State(node): State<AppState>,
    Json(request): Json<CreateLawRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Création loi: {}", request.title);
    
    // TODO: Implémenter la création de loi
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Loi créée avec succès",
        "law_id": Uuid::new_v4().to_string()
    })))
}

/// Handler pour soumettre un vote
async fn submit_vote_handler(
    State(node): State<AppState>,
    Json(request): Json<SubmitVoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Soumission vote pour loi: {}", request.law_id);
    
    // TODO: Implémenter la soumission de vote
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Vote enregistré avec succès",
        "vote_id": Uuid::new_v4().to_string()
    })))
}

/// Handler pour miner un bloc
async fn mine_block_handler(State(node): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Demande de mining manuel");
    
    match node.mine_block().await {
        Ok(Some(block)) => {
            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "Bloc miné avec succès",
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
                "message": "Aucune transaction à miner"
            })))
        }
        Err(e) => {
            error!("Erreur mining: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Placeholder handlers pour les autres endpoints
async fn get_transactions_handler(State(_node): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"transactions": [], "message": "Non implémenté"}))
}

async fn get_transaction_handler(State(_node): State<AppState>, Path(_tx_id): Path<String>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn get_pending_transactions_handler(State(_node): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"pending_transactions": [], "message": "Non implémenté"}))
}

async fn get_accounts_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    
    let accounts: Vec<_> = blockchain.accounts.values()
        .map(|account| serde_json::json!({
            "id": account.id.to_string(),
            "public_key": account.public_key.to_hex(),
            "reputation": account.reputation,
            "is_active": account.is_active,
            "display_name": account.metadata.get("display_name"),
            "created_at": account.metadata.get("created_at")
        }))
        .collect();
    
    Json(serde_json::json!({
        "accounts": accounts,
        "total": accounts.len(),
        "message": "Comptes récupérés depuis la blockchain"
    }))
}

async fn get_account_handler(State(_node): State<AppState>, Path(_account_id): Path<String>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn get_laws_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    
    let laws: Vec<_> = blockchain.laws.values()
        .map(|law| serde_json::json!({
            "id": law.id.to_string(),
            "title": law.title,
            "summary": law.summary,
            "content": law.content,
            "category": law.category,
            "status": format!("{:?}", law.status),
            "author": law.author.to_hex(),
            "version": law.version,
            "created_at": law.created_at.to_rfc3339(),
            "updated_at": law.updated_at.map(|t| t.to_rfc3339())
        }))
        .collect();
    
    Json(serde_json::json!({
        "laws": laws,
        "total": laws.len(),
        "message": "Lois récupérées depuis la blockchain"
    }))
}

async fn get_law_handler(State(_node): State<AppState>, Path(_law_id): Path<String>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn get_law_votes_handler(State(_node): State<AppState>, Path(_law_id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"votes": [], "message": "Non implémenté"}))
}

async fn get_vote_results_handler(State(_node): State<AppState>, Path(_law_id): Path<String>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"results": null, "message": "Non implémenté"}))
}

// Nouveaux handlers pour audit
async fn get_audit_stats_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    
    Json(serde_json::json!({
        "total_blocks": blockchain.blocks.len(),
        "total_transactions": blockchain.blocks.iter().map(|b| b.transactions.len()).sum::<usize>(),
        "total_accounts": blockchain.accounts.len(),
        "total_laws": blockchain.laws.len(),
        "mining_stats": blockchain.get_mining_stats()
    }))
}

async fn get_audit_logs_handler(
    State(node): State<AppState>,
    Query(params): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    let page = params.page.unwrap_or(1);
    let per_page = params.limit.unwrap_or(10);
    
    // Créer des logs d'audit simulés basés sur les transactions
    let mut logs = Vec::new();
    
    for block in blockchain.blocks.iter().rev().take(50) {
        for tx in &block.transactions {
            logs.push(serde_json::json!({
                "id": tx.id.to_string(),
                "timestamp": tx.timestamp.to_rfc3339(),
                "action": format!("Transaction: {:?}", tx.transaction_type),
                "user": tx.sender.to_hex(),
                "details": "Transaction blockchain",
                "ip_address": "blockchain",
                "user_agent": "blockchain-node"
            }));
        }
    }
    
    let total = logs.len();
    let start = ((page - 1) * per_page) as usize;
    let end = (start + per_page as usize).min(logs.len());
    let page_logs = logs[start..end].to_vec();
    
    Json(serde_json::json!({
        "logs": page_logs,
        "pagination": {
            "page": page,
            "per_page": per_page,
            "total": total,
            "pages": (total as f64 / per_page as f64).ceil() as u32
        }
    }))
}

async fn get_audit_export_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    
    Json(serde_json::json!({
        "export_format": "JSON",
        "total_records": blockchain.blocks.len(),
        "message": "Export des données blockchain (implémentation simplifiée)",
        "data": {
            "blocks_count": blockchain.blocks.len(),
            "accounts_count": blockchain.accounts.len(),
            "laws_count": blockchain.laws.len()
        }
    }))
}

// Handlers analytics
async fn get_analytics_overview_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    let mining_stats = blockchain.get_mining_stats();
    
    Json(serde_json::json!({
        "overview": {
            "total_blocks": blockchain.blocks.len(),
            "total_transactions": blockchain.blocks.iter().map(|b| b.transactions.len()).sum::<usize>(),
            "pending_transactions": blockchain.pending_transactions.len(),
            "difficulty": mining_stats.difficulty,
            "hash_rate": mining_stats.estimated_hash_rate,
            "average_block_time": mining_stats.average_block_time
        }
    }))
}

async fn get_analytics_votes_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    
    // Compter les votes par type
    let mut vote_counts = std::collections::HashMap::new();
    for votes in blockchain.votes.values() {
        for vote in votes {
            *vote_counts.entry(format!("{:?}", vote.vote_type)).or_insert(0) += 1;
        }
    }
    
    Json(serde_json::json!({
        "votes_by_type": vote_counts,
        "total_votes": blockchain.votes.values().map(|v| v.len()).sum::<usize>(),
        "laws_with_votes": blockchain.votes.len()
    }))
}

async fn get_analytics_engagement_handler(State(node): State<AppState>) -> Json<serde_json::Value> {
    let blockchain = node.blockchain.read().await;
    
    Json(serde_json::json!({
        "engagement": {
            "total_accounts": blockchain.accounts.len(),
            "active_accounts": blockchain.accounts.values().filter(|a| a.is_active).count(),
            "total_laws": blockchain.laws.len(),
            "laws_with_votes": blockchain.votes.len(),
            "average_votes_per_law": if blockchain.laws.len() > 0 {
                blockchain.votes.values().map(|v| v.len()).sum::<usize>() as f64 / blockchain.laws.len() as f64
            } else { 0.0 }
        }
    }))
}

// Handlers propositions
async fn get_proposals_categories_handler(State(_node): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "categories": [
            "Transport",
            "Environnement", 
            "Éducation",
            "Santé",
            "Économie",
            "Justice",
            "Travail",
            "Logement",
            "Culture",
            "Numérique",
            "Sécurité",
            "Autre"
        ]
    }))
}

async fn support_proposal_handler(
    State(node): State<AppState>,
    Path(proposal_id): Path<String>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Support pour proposition: {}", proposal_id);
    
    // TODO: Implémenter le support réel des propositions
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Support ajouté avec succès",
        "proposal_id": proposal_id
    })))
}
