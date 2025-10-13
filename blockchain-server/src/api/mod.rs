use std::sync::Arc;
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;
use crate::node::BlockchainNode;

// Import all sub-modules
pub mod types;
pub mod handlers;
pub mod routes;
pub mod issuer_api;

// pub use types::*; // avoid unused import warning
// pub use handlers::*; // unused re-export (commented)

/// Shared state type for the API handlers.
type AppState = Arc<BlockchainNode>;

/// Create the main API router with all routes.
///
/// The returned `Router` is already configured with permissive CORS and
/// includes the issuer, API and RPC route groups.
pub fn create_api_router(node: Arc<BlockchainNode>) -> Router {
    let api_routes = routes::create_api_routes();
    let rpc_routes = routes::create_rpc_routes();

    let issuer_routes = issuer_api::routes();

    Router::new()
        // General info routes
        .route("/", get(handlers::root_handler))
        .route("/health", get(handlers::health_handler))
        .route("/stats", get(handlers::stats_handler))
        .route("/info", get(handlers::blockchain_info_handler))
        // Issuer minimal (Phase 2)
        .nest("/issuer", issuer_routes)
        
        // API routes with /api prefix
        .nest("/api", api_routes)
        
        // RPC routes for direct interaction
        .nest("/rpc", rpc_routes)

        .layer(CorsLayer::permissive())
        .with_state(node)
}

/// Start the API server (used by `main.rs`).
///
/// Binds to the configured address/port and serves the Axum application.
pub async fn start_api_server(node: Arc<BlockchainNode>) -> anyhow::Result<()> {
    use tracing::info;
    let app = create_api_router(node.clone());
    let bind_addr = format!("{}:{}", node.config.bind_address, node.config.api_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("🚀 API Server starting on http://{}", bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}