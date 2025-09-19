use anyhow::Result;
use tokio::signal;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialiser le logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("🚀 Démarrage du serveur blockchain e-gouvernement (version debug)");

    // Test simple HTTP avec axum
    use axum::{response::Html, routing::get, Router};
    
    let app = Router::new()
        .route("/", get(|| async { Html("Hello from Rust Blockchain Server!") }))
        .route("/health", get(|| async { "OK" }));
    
    let bind_addr = "0.0.0.0:8080";
    info!("📡 Tentative d'écoute sur: {}", bind_addr);
    
    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
        Ok(listener) => {
            info!("✅ Listener créé avec succès");
            listener
        }
        Err(e) => {
            error!("❌ Erreur création listener: {}", e);
            return Err(e.into());
        }
    };
    
    info!("🚀 Démarrage du serveur axum...");
    let server_handle = tokio::spawn(async move {
        info!("🔄 Serveur axum en cours de démarrage...");
        if let Err(e) = axum::serve(listener, app).await {
            error!("❌ Erreur serveur HTTP: {}", e);
        } else {
            info!("✅ Serveur axum terminé proprement");
        }
    });

    info!("✅ Serveur démarré et en écoute");
    info!("📡 API disponible sur: http://{}", bind_addr);

    // Attendre un signal d'arrêt (SIGINT ou SIGTERM)
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Signal SIGINT reçu, fermeture du serveur...");
        }
        _ = async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            {
                // Sur Windows, attendre indéfiniment (Docker envoie SIGTERM même sur Windows)
                std::future::pending::<()>().await;
            }
        } => {
            info!("Signal SIGTERM reçu, fermeture du serveur...");
        }
    }

    // Arrêter les services proprement
    info!("Arrêt des services...");
    server_handle.abort();

    info!("🛑 Serveur blockchain arrêté");
    Ok(())
}