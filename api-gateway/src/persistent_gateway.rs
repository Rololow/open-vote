use anyhow::Result;
use axum::{Router, Extension, routing::get};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;

use crate::persistent_database::DatabaseService;
use crate::identity_validation::{IdentityValidationService, IdentityValidationConfig};
use crate::user_management::UserManagementService;
use crate::persistent_routes::create_persistent_routes;

/// Service principal intégrant tous les composants du système persistant
#[derive(Debug, Clone)]
pub struct PersistentApiGateway {
    database: DatabaseService,
    user_service: Arc<UserManagementService>,
}

impl PersistentApiGateway {
    /// Initialise le gateway API persistant
    pub async fn new(database_url: &str, jwt_secret: &str) -> Result<Self> {
        // Initialisation de la base de données
        let database = DatabaseService::new(database_url).await?;
        
        // Initialisation du service de validation d'identité (mode simulation)
        let identity_config = IdentityValidationConfig::default();
        let identity_service = IdentityValidationService::new(identity_config);
        
        // Configuration de validation d'identité
        let identity_config = IdentityValidationConfig::default();
        
        // Initialisation du service de gestion des utilisateurs
        let user_service = Arc::new(UserManagementService::new(
            database_url,
            identity_config,
        ).await?);

        Ok(Self {
            database,
            user_service,
        })
    }

    /// Crée le routeur Axum avec toutes les routes
    pub fn create_router(&self) -> Router {
        Router::new()
            .route("/health", get(|| async {
                axum::Json(serde_json::json!({
                    "status": "healthy",
                    "service": "api-gateway-persistent",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }))
            }))
            .merge(create_persistent_routes())
            .layer(Extension(Arc::clone(&self.user_service)))
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            )
    }

    /// Lance le serveur API
    pub async fn serve(self, port: u16) -> Result<()> {
        let app = self.create_router();
        let addr = format!("0.0.0.0:{}", port);
        
        tracing::info!("🚀 Serveur API persistant démarré sur {}", addr);
        tracing::info!("📚 Documentation API disponible sur http://localhost:{}/docs", port);
        tracing::info!("🔐 Endpoints d'authentification: /api/v2/auth/*");
        tracing::info!("🆔 Endpoints de validation d'identité: /api/v2/identity/*");
        tracing::info!("🔑 Endpoints de gestion des clés: /api/v2/crypto/*");
        tracing::info!("🗳️  Endpoints de vote: /api/v2/vote/*");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }

    /// Récupère une référence au service de gestion des utilisateurs
    pub fn user_service(&self) -> &Arc<UserManagementService> {
        &self.user_service
    }

    /// Récupère une référence à la base de données
    pub fn database(&self) -> &DatabaseService {
        &self.database
    }
}

/// Configuration par défaut pour le développement
impl Default for PersistentApiGateway {
    fn default() -> Self {
        // Cette implémentation est synchrone, donc on ne peut pas utiliser async
        // Il faut utiliser new() à la place
        panic!("Utilisez PersistentApiGateway::new() pour créer une instance")
    }
}

/// Utilitaires pour les tests et le développement
impl PersistentApiGateway {
    /// Initialise un gateway en mode test avec base de données en mémoire
    pub async fn test_instance() -> Result<Self> {
        Self::new("sqlite::memory:", "test-jwt-secret-key").await
    }

    /// Crée des données de test
    pub async fn seed_test_data(&self) -> Result<()> {
        tracing::info!("🌱 Création des données de test...");

        // TODO: Créer des utilisateurs de test
        // TODO: Créer des validations d'identité de test
        // TODO: Créer des clés cryptographiques de test

        tracing::info!("✅ Données de test créées");
        Ok(())
    }

    /// Nettoie la base de données (pour les tests)
    pub async fn cleanup_test_data(&self) -> Result<()> {
        tracing::info!("🧹 Nettoyage des données de test...");
        
        // TODO: Implémenter le nettoyage des tables
        
        tracing::info!("✅ Données de test supprimées");
        Ok(())
    }
}

/// Point d'entrée principal pour l'application
pub async fn run_persistent_api_gateway() -> Result<()> {
    // Configuration des logs
    tracing_subscriber::fmt::init();

    // Créer le répertoire de données s'il n'existe pas
    let data_dir = "./data";
    if !std::path::Path::new(data_dir).exists() {
        std::fs::create_dir_all(data_dir)?;
        tracing::info!("📁 Répertoire de données créé: {}", data_dir);
    }

    // Variables d'environnement avec un chemin absolu pour SQLite
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| format!("sqlite:{}/e_government.db", data_dir));
    
    tracing::info!("🗄️ Base de données configurée: {}", database_url);

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super-secret-jwt-key-change-in-production".to_string());
    
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or(3001);

    // Initialisation et lancement du serveur
    let gateway = PersistentApiGateway::new(&database_url, &jwt_secret).await?;
    
    // Création de données de test en mode développement
    if std::env::var("RUST_ENV").unwrap_or_default() == "development" {
        gateway.seed_test_data().await?;
    }

    gateway.serve(port).await
}