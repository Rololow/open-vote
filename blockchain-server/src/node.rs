use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use common::{Blockchain, BlockchainConfig, Transaction, Block};
use crypto_lib::{KeyPair, PublicKey};
use crate::config::ServerConfig;
use crate::storage::Storage;

/// Record d'identité pour la blockchain
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdentityRecord {
    pub national_id_hash: String,
    pub country_code: String,
    pub personal_info_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub registered_by_user_id: uuid::Uuid,
    pub first_registered_at: chrono::DateTime<chrono::Utc>,
    pub validated_by_nodes: Vec<String>,
}

/// Commentaire d'une proposition (simple, en mémoire)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalComment {
    pub id: Uuid,
    pub author: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub upvotes: u32,
    pub downvotes: u32,
}

/// Proposition citoyenne (simple, en mémoire)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub title: String,
    pub category: String,
    pub description: String,
    pub full_text: String,
    pub estimated_budget: Option<u64>,
    pub implementation_timeline: Option<String>,
    pub tags: Vec<String>,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Statut métier affiché côté UI (ex: "Brouillon", "Collecte signatures", "En révision", "Approuvée", "Rejetée")
    pub status: String,
    pub supporters: u32,
    pub upvotes: u32,
    pub downvotes: u32,
    pub comments: Vec<ProposalComment>,
    pub signatures_required: u32,
}

/// Nœud blockchain principal
pub struct BlockchainNode {
    pub config: ServerConfig,
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub blockchain_config: BlockchainConfig,
    pub storage: Arc<Storage>,
    pub keypair: KeyPair,
    pub is_mining: Arc<RwLock<bool>>,
    pub peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Stockage en mémoire des propositions citoyennes (pour démo)
    pub proposals: Arc<RwLock<Vec<Proposal>>>,
}

impl BlockchainNode {
    /// Validate the optional identity_ref on a transaction against DB and config
    async fn validate_identity_ref(&self, transaction: &Transaction) -> Result<()> {
        if let Some(ref hash_hex) = transaction.identity_ref {
            // Lookup in DB
            let rec = self
                .storage
                .get_identity_commitment_by_hash(hash_hex)
                .await
                .context("db lookup identity commitment")?;
            let Some((did, issuer_did, _issued_at, expires_at, status)) = rec else {
                anyhow::bail!("identity_ref unknown: {}", hash_hex);
            };
            // Status
            if status != "active" {
                anyhow::bail!("identity_ref status not active: {}", status);
            }
            // Expiration (if present and in past)
            if let Some(exp) = expires_at.as_deref() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(exp) {
                    if dt.with_timezone(&chrono::Utc) < chrono::Utc::now() {
                        anyhow::bail!("identity_ref expired");
                    }
                }
            }
            // Allowed issuers
            if !self.config.allowed_issuers_dids.is_empty()
                && !self
                    .config
                    .allowed_issuers_dids
                    .iter()
                    .any(|d| d == &issuer_did)
            {
                anyhow::bail!("identity_ref issuer not allowed");
            }
            info!("identity_ref OK for did={} issuer_did={}", did, issuer_did);
        }
        Ok(())
    }
    /// Crée un nouveau nœud blockchain
    pub async fn new(config: ServerConfig) -> Result<Self> {
        info!("🔧 Initialisation du nœud blockchain {}", config.node_id);
        info!("   📊 Configuration:");
        info!("      - Database URL: {}", config.database_url);
        info!("      - Data Directory: {}", config.data_directory);

        // Initialiser le stockage
        info!("💾 Initialisation du stockage de données...");
        let storage = match Storage::new(&config.database_url).await {
            Ok(storage) => {
                info!("✅ Stockage initialisé avec succès");
                Arc::new(storage)
            }
            Err(e) => {
                tracing::error!("❌ Erreur lors de l'initialisation du stockage: {}", e);
                return Err(e);
            }
        };

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
            peers: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Obtient l'adresse publique du nœud
    pub fn public_key(&self) -> &PublicKey {
        self.keypair.public_key()
    }

    /// Normalise une adresse (ajoute schéma http si absent)
    fn normalize_peer(address: &str) -> String {
        let addr = address.trim();
        if addr.starts_with("http://") || addr.starts_with("https://") {
            addr.to_string()
        } else if addr.contains("://") { // autre schéma
            addr.to_string()
        } else if addr.starts_with("127.0.0.1") || addr.starts_with("localhost") || addr.chars().any(|c| c.is_alphabetic()) {
            format!("http://{}", addr)
        } else {
            format!("http://{}", addr)
        }
    }

    /// Ajoute ou met à jour un pair
    pub async fn add_peer(&self, address: String) {
        let norm = Self::normalize_peer(&address);
        let mut peers = self.peers.write().await;
        let entry = peers.entry(norm.clone()).or_insert(PeerInfo { last_seen: Instant::now() });
        entry.last_seen = Instant::now();
        info!("👥 Pair enregistré: {} ({} pairs connus)", norm, peers.len());
    }

    /// Marque un pair comme vu (pong)
    pub async fn mark_peer_seen(&self, address: &str) {
        let norm = Self::normalize_peer(address);
        if let Some(info) = self.peers.write().await.get_mut(&norm) {
            info.last_seen = Instant::now();
        }
    }

    /// Retourne la liste des pairs (adresses seulement)
    pub async fn get_peers(&self) -> Vec<String> {
        self.peers.read().await.keys().cloned().collect()
    }

    /// Retourne pairs avec métadonnées publiques
    pub async fn get_peers_extended(&self) -> Vec<PeerPublic> {
        let now = Instant::now();
        self.peers.read().await.iter().map(|(addr, meta)| {
            let age = now.duration_since(meta.last_seen).as_secs();
            PeerPublic { address: addr.clone(), last_seen_seconds_ago: age }
        }).collect()
    }

    /// Supprime les pairs obsolètes (> max_age)
    pub async fn prune_stale_peers(&self, max_age: Duration) {
        let now = Instant::now();
        let mut peers = self.peers.write().await;
        let before = peers.len();
        peers.retain(|addr, meta| {
            let keep = now.duration_since(meta.last_seen) <= max_age;
            if !keep { info!("🧹 Pair expiré retiré: {}", addr); }
            keep
        });
        let after = peers.len();
        if after != before { info!("🧹 Nettoyage peers: {} -> {}", before, after); }
    }

    /// Ajoute une transaction à la mempool
    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<()> {
        info!("Soumission transaction: {}", transaction.id);

        // 1) Vérification identité optionnelle si identity_ref est présente
        self.validate_identity_ref(&transaction).await?;

        // 2) Ajouter à la mempool si OK
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

        // 1) Prendre un instantané des transactions en attente
        let pending_snapshot: Vec<Transaction> = {
            let blockchain = self.blockchain.read().await;
            blockchain.pending_transactions.clone()
        };

        // 2) Si aucune transaction, sortir proprement en réinitialisant le flag
        if pending_snapshot.is_empty() {
            info!("Aucune transaction à miner");
            *self.is_mining.write().await = false;
            return Ok(None);
        }

        // 3) Valider identity_ref pour toutes les transactions du snapshot
        use std::collections::HashSet as _HashSet;
        let mut invalid_ids: _HashSet<uuid::Uuid> = _HashSet::new();
        for tx in &pending_snapshot {
            if let Err(e) = self.validate_identity_ref(tx).await {
                warn!("Exclusion tx {} de la mempool (identity_ref invalide): {}", tx.id, e);
                invalid_ids.insert(tx.id);
            }
        }

        // 4) Retirer de la mempool les transactions invalides (si présentes)
        if !invalid_ids.is_empty() {
            let mut blockchain = self.blockchain.write().await;
            let before = blockchain.pending_transactions.len();
            blockchain
                .pending_transactions
                .retain(|tx| !invalid_ids.contains(&tx.id));
            let after = blockchain.pending_transactions.len();
            let removed = before.saturating_sub(after);
            if removed > 0 {
                info!("🧹 Transactions retirées avant minage (identity_ref): {}", removed);
            }
        }

        // 5) Miner le bloc si il reste des transactions
        let result = {
            let mut blockchain = self.blockchain.write().await;
            if blockchain.pending_transactions.is_empty() {
                info!("Aucune transaction valable à miner après filtrage");
                Err(anyhow::anyhow!("no-valid-tx"))
            } else {
                blockchain
                    .mine_block(&self.blockchain_config)
                    .map_err(|e| anyhow::anyhow!(e))
            }
        };

        // Libérer le verrou mining
        *self.is_mining.write().await = false;

        match result {
            Ok(block) => {
                // À ce stade, les transactions ont déjà été pré-validées; revalidation optionnelle omise
                info!("✅ Bloc miné avec succès: {}", block.hash.to_hex());
                info!("   - Numéro: {}", block.header.block_number);
                info!("   - Transactions: {}", block.transactions.len());
                
                // Sauvegarder dans le stockage
                let blockchain = self.blockchain.read().await;
                if let Err(e) = self.storage.save_blockchain(&blockchain).await {
                    error!("Erreur sauvegarde blockchain: {}", e);
                }
                
                // Diffuser le bloc aux autres nœuds
                if let Err(e) = self.broadcast_block(&block).await {
                    warn!("Échec de diffusion du bloc: {}", e);
                }
                
                Ok(Some(block))
            }
            Err(e) => {
                if e.to_string() == "no-valid-tx" {
                    // Rien à miner après filtrage
                    Ok(None)
                } else {
                    error!("Erreur mining: {}", e);
                    Err(e.into())
                }
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
        block
            .verify(Some(blockchain.last_block()))
            .map_err(|e| anyhow::anyhow!(e))?;

        // Politique locale: vérifier identity_ref pour chaque transaction reçue
        for tx in &block.transactions {
            if let Err(e) = self.validate_identity_ref(tx).await {
                warn!("Bloc rejeté: identity_ref invalide pour tx {}: {}", tx.id, e);
                return Ok(false);
            }
        }

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

    /// Diffuse un bloc aux pairs connus
    pub async fn broadcast_block(&self, block: &Block) -> Result<()> {
        let peers = self.get_peers().await;
        if peers.is_empty() { return Ok(()); }

        let client = reqwest::Client::new();
        let payload = serde_json::to_value(block)?;

        for peer in peers {
            let url = format!("{}/p2p/block", peer.trim_end_matches('/'));
            let res = client.post(&url).json(&payload).send().await;
            match res {
                Ok(r) if r.status().is_success() => {
                    info!("📡 Bloc diffusé à {}", url);
                }
                Ok(r) => {
                    warn!("⚠️ Diffusion bloc a échoué vers {}: {}", url, r.status());
                }
                Err(e) => {
                    warn!("⚠️ Erreur réseau lors de la diffusion vers {}: {}", url, e);
                }
            }
        }

        Ok(())
    }

    /// Diffuse une transaction aux pairs connus
    pub async fn broadcast_transaction(&self, tx: &Transaction) -> Result<()> {
        let peers = self.get_peers().await;
        if peers.is_empty() { return Ok(()); }

        let client = reqwest::Client::new();
        let payload = serde_json::to_value(tx)?;

        for peer in peers {
            let url = format!("{}/p2p/tx", peer.trim_end_matches('/'));
            let res = client.post(&url).json(&payload).send().await;
            if let Err(e) = res {
                warn!("⚠️ Erreur réseau lors de la diffusion tx vers {}: {}", url, e);
            }
        }

        Ok(())
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
            connected_peers: self.peers.read().await.len(),
        }
    }

    /// Retourne l'ID du nœud
    pub fn get_node_id(&self) -> String {
        self.config.node_id.clone()
    }

    /// Trouve une identité par son hash (simulation pour le moment)
    pub async fn find_identity_by_hash(&self, _hash: &str) -> Option<IdentityRecord> {
        // TODO: Implémenter la recherche réelle dans la blockchain
        None
    }

    /// Enregistre une nouvelle identité dans la blockchain
    pub async fn register_identity(&self, record: &IdentityRecord) -> Result<()> {
        // TODO: Implémenter l'enregistrement réel dans la blockchain
        info!("Enregistrement d'identité: {}", record.national_id_hash);
        Ok(())
    }

    /// Propage une identité vers les pairs P2P
    pub async fn propagate_identity_to_peers(&self, record: &IdentityRecord) {
        // TODO: Implémenter la propagation P2P
        info!("Propagation d'identité vers les pairs: {}", record.national_id_hash);
    }

    /// Récupère toutes les identités enregistrées
    pub async fn get_all_identities(&self) -> Vec<IdentityRecord> {
        // TODO: Implémenter la récupération réelle depuis la blockchain
        Vec::new()
    }

    // ===========================
    // Propositions (en mémoire)
    // ===========================
    pub async fn add_proposal(&self, mut p: Proposal) {
        // Normaliser quelques champs
        p.tags.retain(|t| !t.trim().is_empty());
        let mut proposals = self.proposals.write().await;
        proposals.push(p);
    }

    pub async fn list_proposals(&self) -> Vec<Proposal> {
        self.proposals.read().await.clone()
    }

    pub async fn get_proposal(&self, id: &Uuid) -> Option<Proposal> {
        self.proposals
            .read()
            .await
            .iter()
            .find(|p| &p.id == id)
            .cloned()
    }

    pub async fn support_proposal(&self, id: &Uuid) -> bool {
        let mut proposals = self.proposals.write().await;
        if let Some(p) = proposals.iter_mut().find(|p| &p.id == id) {
            p.supporters = p.supporters.saturating_add(1);
            true
        } else {
            false
        }
    }
}

/// Métadonnées internes d'un pair
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub last_seen: Instant,
}

/// Métadonnées publiques d'un pair
#[derive(Debug, serde::Serialize)]
pub struct PeerPublic {
    pub address: String,
    pub last_seen_seconds_ago: u64,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    use std::str::FromStr;
    use std::path::PathBuf;
    use std::env as std_env;

    fn make_config_with_db(db_suffix: &str, allowed: Vec<String>) -> ServerConfig {
        let mut cfg = ServerConfig::default();
        let db_path = format!("sqlite://./data/test_identity_{}.db", db_suffix);
        cfg.database_url = db_path;
        cfg.allowed_issuers_dids = allowed;
        cfg
    }

    fn ensure_migrations_env() {
        // Compute workspace migrations dir: parent of this crate joined with "migrations"
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_dir = crate_dir.parent().unwrap_or(&crate_dir);
        let migrations_dir = workspace_dir.join("migrations");
        std_env::set_var("MIGRATIONS_DIR", migrations_dir.to_string_lossy().to_string());
    }

    async fn ensure_minimal_schema(db_url: &str) {
        let opts = SqliteConnectOptions::from_str(db_url).unwrap().create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await.unwrap();
        // blocks table (subset sufficient for load_blockchain query)
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                block_number INTEGER NOT NULL UNIQUE,
                hash TEXT NOT NULL UNIQUE,
                previous_hash TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                block_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // identity_commitments table needed by identity validation
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS identity_commitments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                public_key TEXT NOT NULL,
                did TEXT NOT NULL,
                commitment_hash TEXT NOT NULL UNIQUE,
                issuer_did TEXT NOT NULL,
                issued_at TEXT NOT NULL,
                expires_at TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    fn sample_tx_with_identity(identity_ref: Option<&str>) -> Transaction {
        let kp = crypto_lib::KeyPair::generate();
        // Use CreateAccount to avoid dependency on existing on-chain state during mining
        let account = common::Account::new(kp.public_key().clone());
        let ttype = common::TransactionType::CreateAccount(account);
        let mut tx = Transaction::new(ttype, kp.public_key().clone(), kp.sign(b"temp"), 1, 0);
        // Re-signer with proper message format
        let sign_msg = format!(
            "TRANSACTION:{}:{}:{}",
            tx.id,
            tx.timestamp,
            tx.data_hash.to_hex()
        );
        tx.signature = kp.sign(sign_msg.as_bytes());
        tx.identity_ref = identity_ref.map(|s| s.to_string());
        tx
    }

    async fn update_status(db_url: &str, commitment_hash: &str, new_status: &str) {
        let opts = SqliteConnectOptions::from_str(db_url).unwrap().create_if_missing(true);
        let pool = SqlitePool::connect_with(opts).await.unwrap();
        sqlx::query("UPDATE identity_commitments SET status = ?1 WHERE commitment_hash = ?2")
            .bind(new_status)
            .bind(commitment_hash)
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn submit_rejects_unknown_identity_ref() {
        ensure_migrations_env();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        let tx = sample_tx_with_identity(Some("deadbeef"));
        let err = node.submit_transaction(tx).await.expect_err("should reject unknown identity_ref");
        assert!(format!("{}", err).contains("unknown"));
    }

    #[tokio::test]
    async fn submit_rejects_disallowed_issuer() {
        ensure_migrations_env();
        let issuer = "did:key:zIssuerX".to_string();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec!["did:key:zOther".to_string()]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        // Insert commitment with issuer not in allowlist
        let (_id, _existed) = node.storage
            .insert_identity_commitment(
                &node.keypair.public_key().to_hex(),
                "did:key:zSubject1",
                "hash_not_allowed",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(&(chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
            )
            .await
            .expect("insert");

        let tx = sample_tx_with_identity(Some("hash_not_allowed"));
        let res = node.submit_transaction(tx).await;
        assert!(res.is_err(), "should reject due to issuer not allowed");
        let msg = format!("{}", res.err().unwrap());
        assert!(msg.contains("issuer not allowed"), "unexpected error: {}", msg);
    }

    #[tokio::test]
    async fn submit_rejects_expired_identity() {
        ensure_migrations_env();
        let issuer = "did:key:zIssuerAllowed".to_string();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![issuer.clone()]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        let past = "2000-01-01T00:00:00Z";
        let (_id, _existed) = node.storage
            .insert_identity_commitment(
                &node.keypair.public_key().to_hex(),
                "did:key:zSubject2",
                "hash_expired",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(past),
            )
            .await
            .expect("insert");

        let tx = sample_tx_with_identity(Some("hash_expired"));
        let res = node.submit_transaction(tx).await;
        assert!(res.is_err(), "should reject expired identity");
        assert!(format!("{}", res.err().unwrap()).contains("expired"));
    }

    #[tokio::test]
    async fn mining_filters_revoked_identity_from_mempool() {
        ensure_migrations_env();
        let issuer = "did:key:zIssuerAllowed".to_string();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![issuer.clone()]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        // Active commitment initially
        let (_id, _existed) = node.storage
            .insert_identity_commitment(
                &node.keypair.public_key().to_hex(),
                "did:key:zSubject3",
                "hash_revokable",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(&(chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
            )
            .await
            .expect("insert");

        // Submit tx referencing active commitment -> accepted into mempool
        let tx = sample_tx_with_identity(Some("hash_revokable"));
        node.submit_transaction(tx).await.expect("accepted initially");

        // Revoke after it is in the mempool
        update_status(&cfg.database_url, "hash_revokable", "revoked").await;

        // Mine -> should filter it out and produce no block
        let mined = node.mine_block().await.expect("mine call");
        assert!(mined.is_none(), "no block should be mined when only invalid tx present");

        // MemPool should be empty now
        let bc = node.blockchain.read().await;
        assert!(bc.pending_transactions.is_empty());
    }

    #[tokio::test]
    async fn mining_keeps_valid_and_drops_invalid_mixed() {
        ensure_migrations_env();
        let issuer = "did:key:zIssuerAllowed".to_string();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![issuer.clone()]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        // Valid commitment
        node.storage
            .insert_identity_commitment(
                &node.keypair.public_key().to_hex(),
                "did:key:zSubject4",
                "hash_valid",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(&(chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
            )
            .await
            .expect("insert valid");

        // Add one valid tx
        let tx_valid = sample_tx_with_identity(Some("hash_valid"));
        node.submit_transaction(tx_valid).await.expect("valid accepted");

        // Add another tx that will become invalid (revoked)
        node.storage
            .insert_identity_commitment(
                &node.keypair.public_key().to_hex(),
                "did:key:zSubject5",
                "hash_to_revoke",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(&(chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339()),
            )
            .await
            .expect("insert revoke target");
        let tx_revokable = sample_tx_with_identity(Some("hash_to_revoke"));
        node.submit_transaction(tx_revokable).await.expect("accepted");

        // Revoke one of them
        update_status(&cfg.database_url, "hash_to_revoke", "revoked").await;

        // Mine -> should produce a block with only the valid tx
        let mined = node.mine_block().await.expect("mine call");
        let block = mined.expect("block expected");
        assert_eq!(block.transactions.len(), 1, "only one valid tx should be mined");
        // Pending should be empty and the mined tx should be the valid one (hash_valid)
        let bc = node.blockchain.read().await;
        assert!(bc.pending_transactions.is_empty());
        // Can't easily check identity_ref here without cloning; ensure the tx in block had identity_ref Some("hash_valid")
        assert_eq!(block.transactions[0].identity_ref.as_deref(), Some("hash_valid"));
    }
}
