use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};
use tokio::time::{sleep, Duration};

use crate::node::BlockchainNode;

/// Service de consensus pour la blockchain
pub struct ConsensusService {
    node: Arc<BlockchainNode>,
    is_running: bool,
}

impl ConsensusService {
    pub fn new(node: Arc<BlockchainNode>) -> Self {
        Self {
            node,
            is_running: false,
        }
    }

    /// Démarre la boucle de consensus
    pub async fn start(&mut self) -> Result<()> {
        info!("🔄 Démarrage du service de consensus");
        self.is_running = true;

        while self.is_running {
            // Vérifier s'il faut miner un nouveau bloc
            if self.should_mine_block().await {
                if let Err(e) = self.attempt_mining().await {
                    warn!("Erreur during mining: {}", e);
                }
            }

            // Attendre avant la prochaine itération
            sleep(Duration::from_secs(10)).await;
        }

        info!("Service de consensus arrêté");
        Ok(())
    }

    /// Arrête le service
    pub fn stop(&mut self) {
        info!("Arrêt du service de consensus demandé");
        self.is_running = false;
    }

    /// Détermine s'il faut miner un nouveau bloc
    async fn should_mine_block(&self) -> bool {
        if !self.node.config.enable_mining {
            return false;
        }

        let blockchain_info = self.node.get_blockchain_info().await;
        
        // Ne pas miner si déjà en cours
        if blockchain_info.is_mining {
            return false;
        }

        // Miner s'il y a des transactions en attente
        blockchain_info.pending_transactions > 0
    }

    /// Tente de miner un nouveau bloc
    async fn attempt_mining(&self) -> Result<()> {
        info!("🔨 Tentative de mining d'un nouveau bloc");
        
        match self.node.mine_block().await? {
            Some(block) => {
                info!("✅ Bloc miné avec succès: {}", block.hash.to_hex());
                
                // TODO: Diffuser le bloc aux autres nœuds
                self.broadcast_block(block).await?;
            }
            None => {
                info!("Aucun bloc à miner pour le moment");
            }
        }

        Ok(())
    }

    /// Diffuse un bloc aux autres nœuds (placeholder)
    async fn broadcast_block(&self, _block: common::Block) -> Result<()> {
        // TODO: Implémenter la diffusion P2P
        info!("📡 Diffusion du bloc aux pairs (non implémenté)");
        Ok(())
    }
}

/// Point d'entrée pour le service de consensus
pub async fn start_consensus_service(node: Arc<BlockchainNode>) -> Result<()> {
    let mut service = ConsensusService::new(node);
    service.start().await
}
