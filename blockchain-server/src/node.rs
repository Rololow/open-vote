use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, Context};
use tracing::{info, warn, error};

use common::{Blockchain, BlockchainConfig, Transaction, Block, Account, Law, Vote};
use crypto_lib::{KeyPair, PublicKey};
use crate::config::ServerConfig;
use crate::storage::Storage;

/// Nœud blockchain principal
pub struct BlockchainNode {
    pub config: ServerConfig,
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub blockchain_config: BlockchainConfig,
    pub storage: Arc<Storage>,
    pub keypair: KeyPair,
    pub is_mining: Arc<RwLock<bool>>,
}

impl BlockchainNode {
    /// Crée un nouveau nœud blockchain
    pub async fn new(config: ServerConfig) -> Result<Self> {
        info!("Initialisation du nœud blockchain {}", config.node_id);

        // Initialiser le stockage
        let storage = Arc::new(Storage::new(&config.database_url).await?);
        info!("Stockage initialisé");

        // Générer ou charger la paire de clés du nœud
        let keypair = KeyPair::generate();
        info!("Paire de clés générée: {}", keypair.public_key().to_hex());

        // Initialiser ou charger la blockchain
        let blockchain = match storage.load_blockchain().await? {
            Some(loaded_blockchain) => {
                info!("Blockchain chargée depuis le stockage");
                Arc::new(RwLock::new(loaded_blockchain))
            }
            None => {
                info!("Création d'une nouvelle blockchain");
                let new_blockchain = Blockchain::new();
                storage.save_blockchain(&new_blockchain).await?;
                Arc::new(RwLock::new(new_blockchain))
            }
        };

        let blockchain_config = BlockchainConfig::default();

        Ok(Self {
            config,
            blockchain,
            blockchain_config,
            storage,
            keypair,
            is_mining: Arc::new(RwLock::new(false)),
        })
    }

    /// Obtient l'adresse publique du nœud
    pub fn public_key(&self) -> &PublicKey {
        self.keypair.public_key()
    }

    /// Ajoute une transaction à la mempool
    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<()> {
        info!("Soumission transaction: {}", transaction.id);
        
        let mut blockchain = self.blockchain.write().await;
        blockchain.add_pending_transaction(transaction)
            .context("Erreur ajout transaction")?;
        
        info!("Transaction ajoutée à la mempool");
        Ok(())
    }

    /// Mine un nouveau bloc
    pub async fn mine_block(&self) -> Result<Option<Block>> {
        let mut is_mining = self.is_mining.write().await;
        if *is_mining {
            warn!("Mining déjà en cours");
            return Ok(None);
        }
        *is_mining = true;
        drop(is_mining);

        info!("🔨 Début du mining d'un nouveau bloc");

        let result = {
            let mut blockchain = self.blockchain.write().await;
            
            if blockchain.pending_transactions.is_empty() {
                info!("Aucune transaction à miner");
                return Ok(None);
            }

            blockchain.mine_block(&self.blockchain_config)
        };

        // Libérer le verrou mining
        *self.is_mining.write().await = false;

        match result {
            Ok(block) => {
                info!("✅ Bloc miné avec succès: {}", block.hash.to_hex());
                info!("   - Numéro: {}", block.header.block_number);
                info!("   - Transactions: {}", block.transactions.len());
                
                // Sauvegarder dans le stockage
                let blockchain = self.blockchain.read().await;
                if let Err(e) = self.storage.save_blockchain(&blockchain).await {
                    error!("Erreur sauvegarde blockchain: {}", e);
                }
                
                Ok(Some(block))
            }
            Err(e) => {
                error!("Erreur mining: {}", e);
                Err(e.into())
            }
        }
    }

    /// Obtient l'état actuel de la blockchain
    pub async fn get_blockchain_info(&self) -> BlockchainInfo {
        let blockchain = self.blockchain.read().await;
        
        BlockchainInfo {
            height: blockchain.height(),
            pending_transactions: blockchain.pending_transactions.len(),
            total_accounts: blockchain.accounts.len(),
            total_laws: blockchain.laws.len(),
            last_block_hash: blockchain.last_block().hash.to_hex(),
            is_mining: *self.is_mining.read().await,
        }
    }

    /// Valide et ajoute un bloc reçu du réseau
    pub async fn receive_block(&self, block: Block) -> Result<bool> {
        info!("Réception d'un nouveau bloc: {}", block.hash.to_hex());
        
        let mut blockchain = self.blockchain.write().await;
        
        // Vérifier que c'est le prochain bloc attendu
        let expected_height = blockchain.height() + 1;
        if block.header.block_number != expected_height {
            warn!("Bloc rejeté: hauteur incorrecte {} (attendu {})", 
                  block.header.block_number, expected_height);
            return Ok(false);
        }

        // Vérifier la validité du bloc
        block.verify(Some(blockchain.last_block()))?;

        // Les transactions sont déjà appliquées lors de la validation du bloc
        // Pas besoin de les appliquer à nouveau ici

        // Ajouter le bloc
        blockchain.blocks.push(block.clone());
        
        // Nettoyer les transactions traitées
        let processed_ids: std::collections::HashSet<_> = 
            block.transactions.iter().map(|tx| tx.id).collect();
        blockchain.pending_transactions.retain(|tx| !processed_ids.contains(&tx.id));

        info!("✅ Bloc accepté et ajouté à la chaîne");
        
        // Sauvegarder
        if let Err(e) = self.storage.save_blockchain(&blockchain).await {
            error!("Erreur sauvegarde: {}", e);
        }

        Ok(true)
    }

    /// Obtient les statistiques du nœud
    pub async fn get_stats(&self) -> NodeStats {
        let blockchain = self.blockchain.read().await;
        let blockchain_info = self.get_blockchain_info().await;
        
        NodeStats {
            node_id: self.config.node_id.clone(),
            public_key: self.public_key().to_hex(),
            blockchain_info,
            uptime_seconds: 0, // TODO: calculer le temps de fonctionnement
            connected_peers: 0, // TODO: compter les pairs connectés
        }
    }
}

/// Informations sur l'état de la blockchain
#[derive(Debug, serde::Serialize)]
pub struct BlockchainInfo {
    pub height: u64,
    pub pending_transactions: usize,
    pub total_accounts: usize,
    pub total_laws: usize,
    pub last_block_hash: String,
    pub is_mining: bool,
}

/// Statistiques du nœud
#[derive(Debug, serde::Serialize)]
pub struct NodeStats {
    pub node_id: String,
    pub public_key: String,
    pub blockchain_info: BlockchainInfo,
    pub uptime_seconds: u64,
    pub connected_peers: usize,
}
