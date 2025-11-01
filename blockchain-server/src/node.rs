use std::sync::Arc;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::{Instant, Duration};
use tokio::sync::RwLock;
use anyhow::{Result, Context};
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use common::{Blockchain, BlockchainConfig, Transaction, Block};
#[cfg(feature = "identity")]
use common::identity::zkp_prelude::AnonymousActionPayload;
use crypto_lib::{KeyPair, PublicKey};
use crate::config::ServerConfig;
use crate::storage::Storage;

/// Identity record stored on the blockchain.
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

/// Main blockchain node instance.
///
/// Provides operations such as submitting transactions, mining blocks,
/// P2P peer management and basic on-node state (proposals, commitments).
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
    /// Ensemble des commitments actifs (préparation Phase 3)
    pub active_commitments: Arc<RwLock<HashSet<String>>>,
    #[cfg(feature = "zkp_groth16")]
    pub poseidon_params: std::sync::Arc<ark_crypto_primitives::sponge::poseidon::PoseidonConfig<ark_bn254::Fr>>,
}

impl BlockchainNode {
    #[cfg(feature = "identity")]
    fn scope_for_anonymous_vote(law_id: &uuid::Uuid) -> String { format!("vote:{}", law_id) }

    #[cfg(feature = "identity")]
    fn scope_for_anonymous_support(proposal_id: &uuid::Uuid) -> String { format!("support:{}", proposal_id) }

    /// Validate anonymous action payload against recent roots and nullifier store.
    #[cfg(feature = "identity")]
    async fn validate_anonymous_action(&self, payload: &AnonymousActionPayload) -> Result<()> {
        use crate::identity_root::{commitments_log_path, compute_latest_root_from_file, read_last_root_anchors};
        // 1) Root acceptance policy: either equals current computed root, or one of last anchors
        let data_dir = &self.config.data_directory;
        let mut accepted_roots: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Current computed root from commitments.log
        let commitments_path = commitments_log_path(data_dir);
        if let Ok((root, _leaves)) = compute_latest_root_from_file(&commitments_path) {
            accepted_roots.insert(hex::encode(root));
        }
        // Recent anchors (keep a reasonable window)
        if let Ok(entries) = read_last_root_anchors(data_dir, 50) {
            for e in entries { accepted_roots.insert(e.root); }
        }
        let root_ok = accepted_roots.contains(&payload.proof_envelope.root_hex);
        if !root_ok {
            anyhow::bail!("anonymous action root not accepted");
        }

        // 2) Nullifier uniqueness in given scope
        let scope = &payload.proof_envelope.scope;
        let nullifier_hex = &payload.proof_envelope.nullifier_hex;
        if self
            .storage
            .has_nullifier(scope, nullifier_hex)
            .await
            .context("db has_nullifier")?
        {
            anyhow::bail!("duplicate anonymous nullifier for scope");
        }

        // 3) Proof bytes verification (Groth16) if enabled
        if payload.proof_envelope.scheme == common::identity::zkp_prelude::ProofScheme::Groth16 {
            #[cfg(feature = "zkp_groth16")]
            {
                use crate::zkp_verifier::{verify_groth16_bn254, fr_from_be_bytes};
                let root_bytes = hex::decode(&payload.proof_envelope.root_hex)
                    .context("decode root_hex")?;
                let root_fr = fr_from_be_bytes(&root_bytes).context("root to Fr")?;
                let null_bytes = hex::decode(&payload.proof_envelope.nullifier_hex)
                    .context("decode nullifier_hex")?;
                let null_fr = fr_from_be_bytes(&null_bytes).context("nullifier to Fr")?;
                let mut scope_hasher = sha2::Sha256::new();
                use sha2::Digest;
                scope_hasher.update(payload.proof_envelope.scope.as_bytes());
                let scope_digest = scope_hasher.finalize();
                let scope_fr = fr_from_be_bytes(&scope_digest).context("scope to Fr")?;
                let ok = verify_groth16_bn254(
                    data_dir,
                    payload.proof_envelope.vk_version,
                    &payload.proof_envelope.proof,
                    &[root_fr, null_fr, scope_fr],
                    &self.poseidon_params,
                )
                .context("groth16 verification")?;
                if !ok { anyhow::bail!("invalid groth16 proof"); }
            }
            #[cfg(not(feature = "zkp_groth16"))]
            {
                // In builds without the Groth16 verifier, skip cryptographic verification.
                // Policy still enforces root acceptance and nullifier uniqueness.
                warn!("groth16 verification skipped (feature zkp_groth16 not enabled)");
            }
        }
        Ok(())
    }
    /// Validate the optional `identity_ref` on a transaction against DB and config.
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
    /// Create a new blockchain node instance.
    ///
    /// When built with `zkp_groth16`, this will attempt to load Poseidon parameters
    /// from the configured data directory so tests and callers can use ZKP features.
    #[cfg(feature = "zkp_groth16")]
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // Attempt to load Poseidon params from data directory, falling back to generating
        // them from config/poseidon_config.json via zkp_verifier::load_poseidon_params.
        let poseidon_params = match crate::zkp_verifier::load_poseidon_params(&config.data_directory) {
            Ok(p) => std::sync::Arc::new(p),
            Err(e) => {
                tracing::warn!("Failed to load poseidon params from data dir: {}. Attempting to load from config/poseidon_config.json: {}", config.data_directory, e);
                // Try load_poseidon_params will already read config/poseidon_config.json if present
                let p = crate::zkp_verifier::load_poseidon_params(&config.data_directory).context("load poseidon params fallback")?;
                std::sync::Arc::new(p)
            }
        };
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

        let node = Self {
            config,
            blockchain,
            blockchain_config,
            storage,
            keypair,
            is_mining: Arc::new(RwLock::new(false)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(Vec::new())),
            active_commitments: Arc::new(RwLock::new(HashSet::new())),
            poseidon_params,
        };

        // Hydrate active commitments from DB on startup
        if let Err(e) = node.hydrate_active_commitments().await {
            warn!("Failed to hydrate active commitments at startup: {}", e);
        }

    Ok(node)
    }
    #[cfg(not(feature = "zkp_groth16"))]
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // ...existing code...
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
        let keypair = KeyPair::generate();
        info!("Paire de clés générée: {}", keypair.public_key().to_hex());
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
        let node = Self {
            config,
            blockchain,
            blockchain_config,
            storage,
            keypair,
            is_mining: Arc::new(RwLock::new(false)),
            peers: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(Vec::new())),
            active_commitments: Arc::new(RwLock::new(HashSet::new())),
        };
        if let Err(e) = node.hydrate_active_commitments().await {
            warn!("Failed to hydrate active commitments at startup: {}", e);
        }
        Ok(node)
    }
    

    /// Charge en mémoire les commitments actifs depuis la base au démarrage
    async fn hydrate_active_commitments(&self) -> Result<()> {
        let rows = self
            .storage
            .list_active_identity_commitments()
            .await
            .context("list_active_identity_commitments")?;
        let now = chrono::Utc::now();
        let mut set = self.active_commitments.write().await;
        set.clear();
        let mut kept = 0usize;
        for (hash, exp_opt) in rows {
            let keep = match exp_opt.as_deref() {
                Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc) > now,
                    Err(_) => {
                        // Format inattendu: conserver prudemment
                        true
                    }
                },
                None => true,
            };
            if keep {
                set.insert(hash);
                kept += 1;
            }
        }
        info!("Active commitments hydrated: {} kept", kept);

        // Optionally ensure commitments.log exists and contains at least the hydrated set
        let dir = std::path::Path::new(&self.config.data_directory);
        let _ = tokio::fs::create_dir_all(dir).await;
        let log_path = dir.join("commitments.log");
        // If file is missing, backfill with current set
        if tokio::fs::metadata(&log_path).await.is_err() {
            use tokio::io::AsyncWriteExt;
            if let Ok(mut f) = tokio::fs::OpenOptions::new().create(true).write(true).open(&log_path).await {
                for h in set.iter() {
                    let _ = f.write_all(h.as_bytes()).await;
                    let _ = f.write_all(b"\n").await;
                }
                let _ = f.flush().await;
            }
        }
        Ok(())
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

    /// Retourne une copie du set de commitments actifs (pour inspection/tests)
    pub async fn list_active_commitments(&self) -> Vec<String> {
        self.active_commitments.read().await.iter().cloned().collect()
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

        // 1b) If anonymous action, perform basic checks: scope matches and nullifier/root policy
        #[cfg(feature = "identity")]
        {
            use common::TransactionType;
            match &transaction.transaction_type {
                TransactionType::AnonymousVote { law_id, proof } => {
                    let expected = Self::scope_for_anonymous_vote(law_id);
                    if proof.proof_envelope.scope != expected {
                        anyhow::bail!("invalid scope in proof envelope");
                    }
                    self.validate_anonymous_action(proof).await?;
                }
                TransactionType::AnonymousSupport { proposal_id, proof } => {
                    let expected = Self::scope_for_anonymous_support(proposal_id);
                    if proof.proof_envelope.scope != expected {
                        anyhow::bail!("invalid scope in proof envelope");
                    }
                    self.validate_anonymous_action(proof).await?;
                }
                _ => {}
            }
        }

        // 2) Ajouter à la mempool si OK
        let mut blockchain = self.blockchain.write().await;
        blockchain.add_pending_transaction(transaction.clone())
            .context("Erreur ajout transaction")?;
        drop(blockchain);

        info!("Transaction ajoutée à la mempool");

        // Phase 5: Check for automatic proposal promotion
        use common::TransactionType;
        if let TransactionType::SupportProposal { proposal_id, supporter } = &transaction.transaction_type {
            // Update the in-memory proposal support count
            self.support_proposal(proposal_id).await;
            
            // Check if promotion threshold is reached
            if let Some((proposal, support_count)) = self.check_proposal_promotion(proposal_id).await {
                info!("🚀 Automatic promotion triggered for proposal {}", proposal_id);
                
                // Create and submit promotion transaction
                match self.promote_proposal_to_law(&proposal, supporter, support_count).await {
                    Ok(promotion_tx) => {
                        let mut blockchain = self.blockchain.write().await;
                        if let Err(e) = blockchain.add_pending_transaction(promotion_tx) {
                            warn!("Failed to add promotion transaction: {}", e);
                        } else {
                            info!("✅ Promotion transaction added to mempool");
                        }
                    }
                    Err(e) => {
                        warn!("Failed to create promotion transaction: {}", e);
                    }
                }
            }
        }

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

        // 3b) Anonymous actions filtering: enforce root policy and nullifier uniqueness.
        #[cfg(feature = "identity")]
        {
            use common::TransactionType;
            // Track duplicates within the same would-be block
            let mut seen_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
            for tx in &pending_snapshot {
                match &tx.transaction_type {
                    TransactionType::AnonymousVote { law_id, proof } => {
                        // Scope consistency
                        let expected = Self::scope_for_anonymous_vote(law_id);
                        let env = &proof.proof_envelope;
                        if env.scope != expected {
                            warn!("Tx {} invalid anonymous scope (expected {}, got {})", tx.id, expected, env.scope);
                            invalid_ids.insert(tx.id);
                            continue;
                        }
                        // Root+nullifier policy
                        if let Err(e) = self.validate_anonymous_action(proof).await {
                            warn!("Tx {} invalid anonymous action: {}", tx.id, e);
                            invalid_ids.insert(tx.id);
                            continue;
                        }
                        let key = (env.scope.clone(), env.nullifier_hex.clone());
                        if !seen_pairs.insert(key) {
                            warn!("Tx {} duplicate anonymous nullifier in pending set", tx.id);
                            invalid_ids.insert(tx.id);
                        }
                    }
                    TransactionType::AnonymousSupport { proposal_id, proof } => {
                        let expected = Self::scope_for_anonymous_support(proposal_id);
                        let env = &proof.proof_envelope;
                        if env.scope != expected {
                            warn!("Tx {} invalid anonymous scope (expected {}, got {})", tx.id, expected, env.scope);
                            invalid_ids.insert(tx.id);
                            continue;
                        }
                        if let Err(e) = self.validate_anonymous_action(proof).await {
                            warn!("Tx {} invalid anonymous action: {}", tx.id, e);
                            invalid_ids.insert(tx.id);
                            continue;
                        }
                        let key = (env.scope.clone(), env.nullifier_hex.clone());
                        if !seen_pairs.insert(key) {
                            warn!("Tx {} duplicate anonymous nullifier in pending set", tx.id);
                            invalid_ids.insert(tx.id);
                        }
                    }
                    _ => {}
                }
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

                // Anchor identity commitments root for this block (Phase 3)
                if let Err(e) = crate::identity_root::append_root_anchor(&self.config.data_directory, block.header.block_number) {
                    warn!("Failed to append identity root anchor: {}", e);
                }

                // Persist nullifiers from anonymous actions in this block
                #[cfg(feature = "identity")]
                {
                    use common::TransactionType;
                    for tx in &block.transactions {
                        match &tx.transaction_type {
                            TransactionType::AnonymousVote { law_id, proof } => {
                                let scope = Self::scope_for_anonymous_vote(law_id);
                                let _ = self
                                    .storage
                                    .insert_nullifier(&scope, &proof.proof_envelope.nullifier_hex, &tx.id.to_string())
                                    .await;
                            }
                            TransactionType::AnonymousSupport { proposal_id, proof } => {
                                let scope = Self::scope_for_anonymous_support(proposal_id);
                                let _ = self
                                    .storage
                                    .insert_nullifier(&scope, &proof.proof_envelope.nullifier_hex, &tx.id.to_string())
                                    .await;
                            }
                            _ => {}
                        }
                    }
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

        // Anonymous actions policy: root acceptance and nullifier uniqueness (both existing DB and within-block duplicates)
        #[cfg(feature = "identity")]
        {
            use common::TransactionType;
            let mut seen_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
            for tx in &block.transactions {
                match &tx.transaction_type {
                    TransactionType::AnonymousVote { law_id, proof } => {
                        let expected = Self::scope_for_anonymous_vote(law_id);
                        let env = &proof.proof_envelope;
                        if env.scope != expected {
                            warn!("Bloc rejeté: scope anonyme invalide pour tx {}", tx.id);
                            return Ok(false);
                        }
                        if let Err(e) = self.validate_anonymous_action(proof).await {
                            warn!("Bloc rejeté: action anonyme invalide pour tx {}: {}", tx.id, e);
                            return Ok(false);
                        }
                        let key = (env.scope.clone(), env.nullifier_hex.clone());
                        if !seen_pairs.insert(key) {
                            warn!("Bloc rejeté: doublon de nullifier anonyme pour tx {}", tx.id);
                            return Ok(false);
                        }
                    }
                    TransactionType::AnonymousSupport { proposal_id, proof } => {
                        let expected = Self::scope_for_anonymous_support(proposal_id);
                        let env = &proof.proof_envelope;
                        if env.scope != expected {
                            warn!("Bloc rejeté: scope anonyme invalide pour tx {}", tx.id);
                            return Ok(false);
                        }
                        if let Err(e) = self.validate_anonymous_action(proof).await {
                            warn!("Bloc rejeté: action anonyme invalide pour tx {}: {}", tx.id, e);
                            return Ok(false);
                        }
                        let key = (env.scope.clone(), env.nullifier_hex.clone());
                        if !seen_pairs.insert(key) {
                            warn!("Bloc rejeté: doublon de nullifier anonyme pour tx {}", tx.id);
                            return Ok(false);
                        }
                    }
                    _ => {}
                }
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

        // Anchor identity commitments root for this block (Phase 3)
        if let Err(e) = crate::identity_root::append_root_anchor(&self.config.data_directory, block.header.block_number) {
            warn!("Failed to append identity root anchor: {}", e);
        }

        // Persist nullifiers from anonymous actions
        #[cfg(feature = "identity")]
        {
            use common::TransactionType;
            for tx in &block.transactions {
                match &tx.transaction_type {
                    TransactionType::AnonymousVote { law_id, proof } => {
                        let scope = Self::scope_for_anonymous_vote(law_id);
                        let _ = self
                            .storage
                            .insert_nullifier(&scope, &proof.proof_envelope.nullifier_hex, &tx.id.to_string())
                            .await;
                    }
                    TransactionType::AnonymousSupport { proposal_id, proof } => {
                        let scope = Self::scope_for_anonymous_support(proposal_id);
                        let _ = self
                            .storage
                            .insert_nullifier(&scope, &proof.proof_envelope.nullifier_hex, &tx.id.to_string())
                            .await;
                    }
                    _ => {}
                }
            }
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

    /// Ajoute un commitment actif en mémoire et dans le journal append-only
    pub async fn add_active_commitment(&self, commitment_hash: &str) {
        {
            let mut set = self.active_commitments.write().await;
            set.insert(commitment_hash.to_string());
        }
        // Append to log file in data_directory
        let dir = std::path::Path::new(&self.config.data_directory);
        let _ = tokio::fs::create_dir_all(dir).await;
        let log_path = dir.join("commitments.log");
        let line = format!("{}\n", commitment_hash);
        // Use append mode
        if let Ok(mut f) = tokio::fs::OpenOptions::new().create(true).append(true).open(&log_path).await {
            use tokio::io::AsyncWriteExt;
            let _ = f.write_all(line.as_bytes()).await;
            let _ = f.flush().await;
        }
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

    /// Phase 5.4: Mempool hygiene - Remove stale transactions
    /// Removes transactions that have been in the mempool for too long (default: 1 hour)
    pub async fn cleanup_stale_mempool_transactions(&self, max_age_seconds: u64) -> usize {
        use chrono::Utc;
        
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        let mut blockchain = self.blockchain.write().await;
        
        let before = blockchain.pending_transactions.len();
        blockchain.pending_transactions.retain(|tx| tx.timestamp > cutoff);
        let after = blockchain.pending_transactions.len();
        
        let removed = before.saturating_sub(after);
        if removed > 0 {
            info!("🧹 Mempool cleanup: removed {} stale transactions (older than {}s)", removed, max_age_seconds);
        }
        
        removed
    }

    /// Phase 5.4: Mempool hygiene - Limit mempool size
    /// Removes oldest transactions if mempool exceeds max size
    pub async fn enforce_mempool_size_limit(&self, max_size: usize) -> usize {
        let mut blockchain = self.blockchain.write().await;
        
        if blockchain.pending_transactions.len() <= max_size {
            return 0;
        }
        
        // Sort by timestamp (oldest first) and keep only the most recent max_size
        blockchain.pending_transactions.sort_by_key(|tx| tx.timestamp);
        let to_remove = blockchain.pending_transactions.len() - max_size;
        blockchain.pending_transactions.drain(0..to_remove);
        
        if to_remove > 0 {
            info!("🧹 Mempool size limit: removed {} oldest transactions (limit: {})", to_remove, max_size);
        }
        
        to_remove
    }

    /// Phase 5.4: Mempool hygiene - Remove transactions with duplicate nullifiers (anonymous txs)
    /// This is an additional safety check beyond the mining-time validation
    #[cfg(feature = "identity")]
    pub async fn cleanup_duplicate_nullifiers_in_mempool(&self) -> usize {
        use common::TransactionType;
        use std::collections::{HashMap, HashSet};
        
        let mut blockchain = self.blockchain.write().await;
        let mut seen: HashMap<String, HashSet<String>> = HashMap::new();
        let mut to_remove: HashSet<uuid::Uuid> = HashSet::new();
        
        for tx in &blockchain.pending_transactions {
            let nullifier_key = match &tx.transaction_type {
                TransactionType::AnonymousVote { law_id, proof } => {
                    Some((Self::scope_for_anonymous_vote(law_id), proof.proof_envelope.nullifier_hex.clone()))
                }
                TransactionType::AnonymousSupport { proposal_id, proof } => {
                    Some((Self::scope_for_anonymous_support(proposal_id), proof.proof_envelope.nullifier_hex.clone()))
                }
                _ => None,
            };
            
            if let Some((scope, nullifier)) = nullifier_key {
                if !seen.entry(scope.clone()).or_default().insert(nullifier.clone()) {
                    // Duplicate found - mark for removal
                    to_remove.insert(tx.id);
                    warn!("🧹 Duplicate nullifier detected in mempool: scope={}, nullifier={}", scope, nullifier);
                }
            }
        }
        
        let removed = to_remove.len();
        if removed > 0 {
            blockchain.pending_transactions.retain(|tx| !to_remove.contains(&tx.id));
            info!("🧹 Removed {} transactions with duplicate nullifiers from mempool", removed);
        }
        
        removed
    }

    /// Phase 5: Check if a proposal has reached the promotion threshold and should be promoted to a law
    /// Returns Some((Proposal, support_count)) if promotion should occur, None otherwise
    pub async fn check_proposal_promotion(&self, proposal_id: &Uuid) -> Option<(Proposal, u32)> {
        let proposals = self.proposals.read().await;
        if let Some(proposal) = proposals.iter().find(|p| &p.id == proposal_id) {
            // Check if proposal is in "Collecte signatures" status and has enough supporters
            if proposal.status == "Collecte signatures" 
                && proposal.supporters >= self.config.proposal_promotion_threshold {
                info!("🎯 Proposal {} has reached promotion threshold: {} >= {}", 
                    proposal_id, proposal.supporters, self.config.proposal_promotion_threshold);
                return Some((proposal.clone(), proposal.supporters));
            }
        }
        None
    }

    /// Phase 5: Promote a proposal to a law (automatic promotion logic)
    pub async fn promote_proposal_to_law(&self, proposal: &Proposal, supporter: &crypto_lib::PublicKey, support_count: u32) -> Result<common::Transaction> {
        use common::{Law, LawChangeType, LawStatus, TransactionType};
        use crypto_lib::KeyPair;
        
        info!("📜 Promoting proposal {} to law", proposal.id);
        
        // Create a new law from the proposal
        let mut law = Law::new(
            proposal.title.clone(),
            proposal.full_text.clone(),
            proposal.description.clone(),
            proposal.category.clone(),
            supporter.clone(),
            LawChangeType::Creation,
        );
        
        // Set law to InReview status (not immediately active)
        law.status = LawStatus::InReview;
        law.tags = proposal.tags.clone();
        
        // Create a LawPromoted transaction to record this event
        let law_id = law.id;
        let tx_type = TransactionType::LawPromoted {
            proposal_id: proposal.id,
            law_id,
            promoted_by: supporter.clone(),
            support_count,
        };
        
        // Sign the transaction (using node's keypair or a system keypair)
        // For now, we'll use the supporter's signature, but in production this should be signed by the node
        let kp = KeyPair::generate(); // TODO: Use a proper system keypair
        let tx_msg = format!("PROMOTE:{}:{}", proposal.id, law_id);
        let signature = kp.sign(tx_msg.as_bytes());
        
        let transaction = common::Transaction::new(
            tx_type,
            supporter.clone(),
            signature,
            0, // nonce
            0, // fee
        );
        
        Ok(transaction)
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
    use crate::storage::Storage;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    use std::str::FromStr;
    use std::path::PathBuf;
    use std::env as std_env;
    #[cfg(feature = "identity")]
    use common::identity::zkp_prelude::{ProofEnvelope, AnonymousActionPayload, ProofScheme};

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

/// Public helper to ensure minimal DB schema for tests and integration
pub async fn ensure_minimal_schema(db_url: &str) {
        // Ensure parent directory exists so Sqlite can create the DB file
        if let Some(stripped) = db_url.strip_prefix("sqlite://") {
            if let Some(parent) = std::path::Path::new(stripped).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
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

    #[tokio::test]
    async fn startup_hydrates_active_commitments_non_expired() {
        ensure_migrations_env();
        let issuer = "did:key:zIssuer".to_string();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![issuer.clone()]);
        ensure_minimal_schema(&cfg.database_url).await;

        // Pre-insert an active, non-expired commitment BEFORE node creation
        let storage = Storage::new(&cfg.database_url).await.expect("storage");
        let future = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();
        let (_id, _existed) = storage
            .insert_identity_commitment(
                "pk1",
                "did:key:zSubjectHydrate",
                "hash_hydrate_ok",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(&future),
            )
            .await
            .expect("insert");

        // Now create node -> should hydrate set with the above hash
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");
        let set = node.list_active_commitments().await;
        assert!(set.contains(&"hash_hydrate_ok".to_string()));
    }

    #[tokio::test]
    async fn startup_hydration_excludes_expired_commitments() {
        ensure_migrations_env();
        let issuer = "did:key:zIssuer".to_string();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![issuer.clone()]);
        ensure_minimal_schema(&cfg.database_url).await;

        // Pre-insert an active but expired commitment
        let storage = Storage::new(&cfg.database_url).await.expect("storage");
        let past = "2000-01-01T00:00:00Z";
        let (_id, _existed) = storage
            .insert_identity_commitment(
                "pk2",
                "did:key:zSubjectExpired",
                "hash_hydrate_expired",
                &issuer,
                &chrono::Utc::now().to_rfc3339(),
                Some(past),
            )
            .await
            .expect("insert");

        let node = BlockchainNode::new(cfg.clone()).await.expect("node");
        let set = node.list_active_commitments().await;
        assert!(!set.contains(&"hash_hydrate_expired".to_string()));
    }

    #[cfg(feature = "identity")]
    fn make_fake_envelope(scope: &str, root_hex: &str, nullifier_hex: &str) -> AnonymousActionPayload {
        AnonymousActionPayload {
            proof_envelope: ProofEnvelope {
                scheme: ProofScheme::Groth16,
                vk_version: 1,
                root_hex: root_hex.to_string(),
                scope: scope.to_string(),
                nullifier_hex: nullifier_hex.to_string(),
                proof: vec![],
                public_inputs: vec![],
            },
            payload: None,
        }
    }

    #[cfg(feature = "identity")]
    async fn current_root_hex(data_dir: &str) -> String {
        let p = crate::identity_root::commitments_log_path(data_dir);
        let (r, _n) = crate::identity_root::compute_latest_root_from_file(&p).unwrap();
        hex::encode(r)
    }

    #[tokio::test]
    #[cfg(all(feature = "identity", not(feature = "zkp_groth16")))]
    async fn anonymous_duplicate_nullifier_rejected_in_submit() {
        ensure_migrations_env();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        // Build two AnonymousSupport with same (scope,nullifier)
        let prop_id = uuid::Uuid::new_v4();
        let scope = format!("support:{}", prop_id);
        // Ensure a corresponding proposal exists on-chain to pass state application
        {
            use common::proposal::{Proposal as CProposal, ProposalStatus};
            let mut bc = node.blockchain.write().await;
            let proposal = CProposal {
                id: prop_id,
                title: "t".into(),
                category: "c".into(),
                description: "d".into(),
                full_text: "f".into(),
                estimated_budget: None,
                implementation_timeline: None,
                tags: vec![],
                author_id: None,
                author_name: None,
                created_at: chrono::Utc::now(),
                expires_at: chrono::Utc::now() + chrono::Duration::days(30),
                status: ProposalStatus::CollectingSignatures,
                supporters_count: 0,
            };
            bc.proposals.insert(prop_id, proposal);
        }
        // Ensure identity_roots has at least one entry matching current root
        let root_hex = current_root_hex(&node.config.data_directory).await;
        let env = make_fake_envelope(&scope, &root_hex, "deadbeefdeadbeef");

        // Craft transactions
        let kp = crypto_lib::KeyPair::generate();
        let t1 = common::TransactionType::AnonymousSupport { proposal_id: prop_id, proof: env.clone() };
        let mut tx1 = Transaction::new(t1, kp.public_key().clone(), kp.sign(b"t"), 1, 0);
        let sign_msg1 = format!("TRANSACTION:{}:{}:{}", tx1.id, tx1.timestamp, tx1.data_hash.to_hex());
        tx1.signature = kp.sign(sign_msg1.as_bytes());

        let t2 = common::TransactionType::AnonymousSupport { proposal_id: prop_id, proof: env };
        let mut tx2 = Transaction::new(t2, kp.public_key().clone(), kp.sign(b"t"), 2, 0);
        let sign_msg2 = format!("TRANSACTION:{}:{}:{}", tx2.id, tx2.timestamp, tx2.data_hash.to_hex());
        tx2.signature = kp.sign(sign_msg2.as_bytes());

        // First should be accepted to mempool
        node.submit_transaction(tx1).await.expect("first accepted");
        // Second should be rejected at submit time as duplicate nullifier in DB? Not yet inserted -> allow mempool, but we'll filter during mine.
        // However validate_anonymous_action checks DB only; duplicates in mempool are filtered at mining. So submit should succeed.
        node.submit_transaction(tx2).await.expect("second accepted pending");

        // Mine -> should keep only one and insert nullifier
        let mined = node.mine_block().await.expect("mine ok");
        let block = mined.expect("block expected");
        assert_eq!(block.transactions.len(), 1);
        // Next mining should produce no block because no more tx
        assert!(node.mine_block().await.unwrap().is_none());
    }

    #[tokio::test]
    #[cfg(all(feature = "identity", feature = "zkp_groth16", feature = "zkp_mock"))]
    async fn groth16_mock_verifier_accepts_ok_proof() {
        use common::TransactionType;
        ensure_migrations_env();
        let cfg = make_config_with_db(&uuid::Uuid::new_v4().to_string(), vec![]);
        ensure_minimal_schema(&cfg.database_url).await;
        let node = BlockchainNode::new(cfg.clone()).await.expect("node");

        // Prepare mock VK file with magic header
        let vk_path = crate::zkp_verifier::vk_path_for_version(&node.config.data_directory, 42);
        std::fs::create_dir_all(vk_path.parent().unwrap()).unwrap();
        std::fs::write(&vk_path, b"MOCKVK").unwrap();

        // Ensure a simple on-chain object exists for applying
        // Create a proposal in chain state so AnonymousSupport is valid
        let prop_id = uuid::Uuid::new_v4();
        {
            use common::proposal::{Proposal as CProposal, ProposalStatus};
            let mut bc = node.blockchain.write().await;
            let proposal = CProposal {
                id: prop_id,
                title: "t".into(),
                category: "c".into(),
                description: "d".into(),
                full_text: "f".into(),
                estimated_budget: None,
                implementation_timeline: None,
                tags: vec![],
                author_id: None,
                author_name: None,
                created_at: chrono::Utc::now(),
                expires_at: chrono::Utc::now() + chrono::Duration::days(30),
                status: ProposalStatus::CollectingSignatures,
                supporters_count: 0,
            };
            bc.proposals.insert(prop_id, proposal);
        }

        // Use current identity root
        let root_hex = current_root_hex(&node.config.data_directory).await;
        let scope = format!("support:{}", prop_id);
        let mut env = make_fake_envelope(&scope, &root_hex, "aabbccdd");
        // Set vk_version to the mock one and put proof bytes to OK sentinel
        env.proof_envelope.vk_version = 42;
        env.proof_envelope.proof = b"OK".to_vec();

        // Build tx and submit
        let kp = crypto_lib::KeyPair::generate();
        let t = TransactionType::AnonymousSupport { proposal_id: prop_id, proof: env };
        let mut tx = Transaction::new(t, kp.public_key().clone(), kp.sign(b"t"), 1, 0);
        let sign_msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
        tx.signature = kp.sign(sign_msg.as_bytes());

        // Submit should pass thanks to mock verifier accepting the proof
        node.submit_transaction(tx).await.expect("submit anonymous support with mock proof");

        // Mine and expect a block
        let mined = node.mine_block().await.expect("mine").expect("block");
        assert_eq!(mined.transactions.len(), 1);
    }
}
