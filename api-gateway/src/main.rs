mod gateway;
mod auth;
mod middleware;
mod proxy;
mod routes;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialiser le logging
    tracing_subscriber::fmt()
        .init();

    info!("🚀 Démarrage de la passerelle API e-gouvernement");

    // Démarrer la passerelle
    gateway::start_gateway().await?;

    Ok(())
}
