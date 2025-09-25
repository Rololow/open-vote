// Serveur blockchain pour le système e-gouvernement
// Version avec base de données SQLite persistante

mod database;
mod migration;
mod security;
mod node;
mod api;
mod network;
mod consensus;
mod config;
mod storage;
mod issuer; // Phase 2: module d'émission VC

use anyhow::Result;
use std::sync::Arc;
use std::env;
use tokio::signal;
use tracing::info;
use serde::Deserialize;

// État partagé de l'application
use config::ServerConfig;
use node::BlockchainNode;

// Structures pour les requêtes API
#[derive(Debug, Deserialize)]
struct VoteRequest {
    voter_id: String,
    vote_type: String,
    comment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SecurityQueryParams {
    security_level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MigrationRequest {
    mock_server_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialiser le logging
    tracing_subscriber::fmt::init();
    info!("🚀 Démarrage du serveur blockchain e-gouvernement avec SQLite");

    // Parse simple CLI args: --migrate to apply migrations only; otherwise optional config file path
    let args: Vec<String> = env::args().collect();
    let mut migrate_only = false;
    let mut config_file: Option<String> = None;
    for arg in args.iter().skip(1) {
        if arg == "--migrate" {
            migrate_only = true;
        } else if config_file.is_none() {
            config_file = Some(arg.clone());
        }
    }

    // Charger la configuration
    let config = if let Some(config_path) = config_file.as_deref() {
        info!("📄 Chargement de la configuration depuis: {}", config_path);
        ServerConfig::load_from_file(config_path).await?
    } else {
        info!("⚙️ Utilisation de la configuration par défaut");
        ServerConfig::load().await?
    };

    // If only migrating, init storage (which runs migrations) and exit
    if migrate_only {
        info!("🧭 Mode migration uniquement activé (--migrate)");
        let storage = crate::storage::Storage::new(&config.database_url).await?;
        // Explicitly drop to close connections before exit
        drop(storage);
        info!("✅ Migrations appliquées. Arrêt.");
        return Ok(());
    }

    // Créer le nœud principal
    info!("🏗️ Création du nœud blockchain...");
    let node = Arc::new(BlockchainNode::new(config.clone()).await?);
    info!("✅ Nœud créé avec succès: {}", node.config.node_id);

    // Démarrer les services (API + consensus) en parallèle
    let api_node = node.clone();
    let consensus_node = node.clone();

    info!("🚀 Démarrage des services...");
    info!("   📡 API Server sur: {}:{}", config.bind_address, config.api_port);
    info!("   🔗 P2P Network sur: {}:{}", config.bind_address, config.p2p_port);

    let api_handle = tokio::spawn(async move {
        info!("🌐 Démarrage du serveur API...");
        let res = api::start_api_server(api_node.clone()).await;
        match &res {
            Ok(_) => info!("⚠️ Tâche API terminée (serveur arrêté proprement ou s'est fermée de manière inattendue)"),
            Err(e) => tracing::error!("❌ Tâche API terminée avec erreur: {e:?}"),
        }
        res
    });
    let consensus_handle = tokio::spawn(async move {
        info!("🎯 Démarrage du service consensus...");
        let res = consensus::start_consensus_service(consensus_node.clone()).await;
        match &res {
            Ok(_) => info!("⚠️ Tâche consensus terminée (loop interrompue)"),
            Err(e) => tracing::error!("❌ Tâche consensus terminée avec erreur: {e:?}"),
        }
        res
    });

    info!("🎉 Tous les services démarrés! Serveur prêt à recevoir des requêtes.");

    // Attendre l'arrêt
    let mut api_handle = api_handle;
    let mut consensus_handle = consensus_handle;
    tokio::pin!(api_handle);
    tokio::pin!(consensus_handle);

    tokio::select! {
        _ = shutdown_signal() => {
            info!("🛑 Signal d'arrêt reçu");
        },
        api_res = &mut api_handle => {
            info!("🛑 Arrêt déclenché car la tâche API s'est terminée");
            match api_res {
                Ok(Ok(_)) => info!("ℹ️ Tâche API terminée sans erreur interne"),
                Ok(Err(e)) => tracing::error!("❌ Erreur interne tâche API: {e:?}"),
                Err(join_err) => tracing::error!("❌ Erreur join tâche API: {join_err:?}"),
            }
        },
        consensus_res = &mut consensus_handle => {
            info!("🛑 Arrêt déclenché car la tâche consensus s'est terminée");
            match consensus_res {
                Ok(Ok(_)) => info!("ℹ️ Tâche consensus terminée sans erreur interne"),
                Ok(Err(e)) => tracing::error!("❌ Erreur interne tâche consensus: {e:?}"),
                Err(join_err) => tracing::error!("❌ Erreur join tâche consensus: {join_err:?}"),
            }
        }
    }
    Ok(())
}

// Signal d'arrêt gracieux
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Installation du gestionnaire Ctrl+C échouée");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Installation du gestionnaire SIGTERM échouée")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("💡 Signal Ctrl+C reçu, arrêt en cours...");
        },
        _ = terminate => {
            info!("💡 Signal SIGTERM reçu, arrêt en cours...");
        },
    }
}

// Les handlers API spécifiques ont été déplacés dans api.rs
