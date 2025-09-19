use anyhow::Result;
use axum::{routing::get, Router};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing::info;
use crypto_lib::KeyPair;

use crate::auth::AuthManager;
use crate::routes::{create_router_with_state, AppState};

pub async fn start_gateway() -> Result<()> {
    // Créer la clé de signature pour les JWT
    let signing_key = KeyPair::generate();
    info!("🔐 Clé de signature JWT générée");

    // Créer le gestionnaire d'authentification
    let auth_manager = AuthManager::new(signing_key, 24); // Tokens valides 24h
    info!("🔑 Gestionnaire d'authentification initialisé");

    // Créer l'état partagé de l'application
    let app_state = AppState::new(auth_manager);
    info!("🏗️ État de l'application créé");

    // Créer le routeur avec toutes les routes sécurisées
    let api_router = create_router_with_state(app_state);

    // Router principal avec la route de santé
    let app = Router::new()
        .route("/", get(|| async { "🚀 API Gateway E-Government - Système d'authentification JWT opérationnel" }))
        .route("/health", get(|| async { 
            axum::Json(serde_json::json!({
                "status": "healthy",
                "service": "api-gateway",
                "version": "1.0.0",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "features": [
                    "JWT Authentication",
                    "Role-Based Access Control",
                    "Ed25519 Cryptographic Signatures",
                    "Multi-user Session Management"
                ]
            }))  
        }))
        .nest("/api", api_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .layer(TraceLayer::new_for_http());

    let bind_addr = "127.0.0.1:9000";
    info!("🌐 API Gateway démarrée sur http://{}", bind_addr);
    info!("📋 Endpoints disponibles :");
    info!("   GET  /                     - Informations du service");
    info!("   GET  /health               - Vérification de santé");
    info!("   POST /api/auth/login       - Connexion utilisateur");
    info!("   POST /api/auth/register    - Inscription utilisateur");
    info!("   GET  /api/auth/profile     - Profil utilisateur (🔒)");
    info!("   POST /api/auth/logout      - Déconnexion (🔒)");
    info!("   GET  /api/laws             - Liste des lois (🔒)");
    info!("   POST /api/laws             - Créer une loi (🔒 CREATE_LAW)");
    info!("   GET  /api/laws/:id         - Détail d'une loi (🔒)");
    info!("   PUT  /api/laws/:id         - Modifier une loi (🔒 MODIFY_LAW)");
    info!("   DELETE /api/laws/:id       - Supprimer une loi (🔒 DELETE_LAW)");
    info!("   POST /api/vote             - Voter sur une loi (🔒 VOTE_ON_LAW)");
    info!("   GET  /api/admin/users      - Liste utilisateurs (🔒 Admin)");
    info!("   GET  /api/stats            - Statistiques (🔒 Auditor+)");
    info!("");
    info!("🔐 Comptes de démonstration :");
    info!("   admin@example.com / admin123    (Administrateur)");
    info!("   marie@example.com / demo123     (Citoyen)");
    info!("   jean@example.com  / demo123     (Représentant)");

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
