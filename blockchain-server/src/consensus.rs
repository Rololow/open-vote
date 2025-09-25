use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};
use tokio::time::{sleep, Duration, Instant};

use crate::node::BlockchainNode;
use common::Block;

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

        info!("🔄 Entrée dans la boucle de consensus");
        
        let gossip_interval = Duration::from_secs(20);
        let prune_interval = Duration::from_secs(60);
        let mut last_gossip = Instant::now();
        let mut last_prune = Instant::now();

        while self.is_running {
            info!("🔍 Vérification du besoin de miner un bloc...");
            
            // Vérifier s'il faut miner un nouveau bloc
            match self.should_mine_block().await {
                true => {
                    info!("✅ Mining requis, tentative...");
                    if let Err(e) = self.attempt_mining().await {
                        warn!("❌ Erreur during mining: {}", e);
                    }
                }
                false => {
                    info!("⏭️ Pas besoin de miner pour le moment");
                }
            }

            // Gossip pairs
            if last_gossip.elapsed() >= gossip_interval {
                last_gossip = Instant::now();
                let peers_snapshot = self.node.get_peers().await;
                if !peers_snapshot.is_empty() {
                    let payload = serde_json::json!({"peers": peers_snapshot});
                    let client = reqwest::Client::new();
                    for peer in self.node.get_peers().await { // relecture volontaire
                        let url = format!("{}/peers/discover", peer.trim_end_matches('/'));
                        let _ = client.post(&url).json(&payload).send().await; // silencieux
                    }
                }
            }

            // Prune pairs
            if last_prune.elapsed() >= prune_interval {
                self.node.prune_stale_peers(Duration::from_secs(180)).await; // TTL: 3 min
                last_prune = Instant::now();
            }

            info!("⏰ Attente avant la prochaine vérification (10s)");
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
        info!("🔧 Vérification de la configuration de mining...");
        
        if !self.node.config.enable_mining {
            info!("⛔ Mining désactivé dans la configuration");
            return false;
        }
        info!("✅ Mining activé dans la configuration");

        info!("📊 Récupération des informations de la blockchain...");
        let blockchain_info = match self.node.get_blockchain_info().await {
            info => {
                info!("📈 Infos blockchain: mining={}, transactions_pending={}", 
                     info.is_mining, info.pending_transactions);
                info
            }
        };
        
        // Ne pas miner si déjà en cours
        if blockchain_info.is_mining {
            info!("🔄 Mining déjà en cours, attente...");
            return false;
        }

        let should_mine = blockchain_info.pending_transactions > 0;
        info!("🎯 Décision de mining: {}", should_mine);
        
        should_mine
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
    async fn broadcast_block(&self, block: Block) -> Result<()> {
        self.node.broadcast_block(&block).await?;
        Ok(())
    }
}

/// Point d'entrée pour le service de consensus
pub async fn start_consensus_service(node: Arc<BlockchainNode>) -> Result<()> {
    let mut service = ConsensusService::new(node);
    service.start().await
}
