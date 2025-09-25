use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::{Block, Transaction, Account, Law, Vote, BlockchainError, Result, proposal::Proposal};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crypto_lib::Signature;

/// Attestation de vérification liée à un compte (sans PII)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedAttestation {
    pub id: Uuid,
    pub account_id: Uuid,
    pub national_id_hash: String,
    pub country_code: String,
    pub scheme: String,
    pub issuer_id: String,
    pub evidence_hash: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub issuer_signature: Signature,
}

/// État de la blockchain e-gouvernement
#[derive(Debug, Clone)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub accounts: HashMap<uuid::Uuid, Account>,
    pub laws: HashMap<uuid::Uuid, Law>,
    pub votes: HashMap<uuid::Uuid, Vec<Vote>>, // Votes par loi
    pub proposals: HashMap<uuid::Uuid, Proposal>,
    /// Ensemble des supporters par proposition (clé publique unique)
    pub proposal_supporters: HashMap<uuid::Uuid, HashSet<String>>, // hex(pubkey)
    /// Attestations de vérification actives par compte (plusieurs attestations possibles)
    pub verifications_by_account: HashMap<uuid::Uuid, Vec<VerifiedAttestation>>,
    /// Index des attestations par identifiant d'attestation
    pub verifications_by_id: HashMap<uuid::Uuid, VerifiedAttestation>,
    /// Index d'unicité identité → compte
    pub identity_index: HashMap<String, uuid::Uuid>,
    pub pending_transactions: Vec<Transaction>,
    pub difficulty: u32,
    pub block_reward: u64,
}

/// Configuration de la blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub max_block_size: usize,
    pub max_transactions_per_block: usize,
    pub target_block_time_seconds: u64,
    pub difficulty_adjustment_interval: u64,
    pub minimum_vote_duration_hours: u32,
    pub maximum_vote_duration_hours: u32,
    pub quorum_percentage: f64,
    pub supermajority_threshold: f64,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            max_block_size: 1_000_000, // 1MB
            max_transactions_per_block: 1000,
            target_block_time_seconds: 300, // 5 minutes
            difficulty_adjustment_interval: 144, // ~12 heures
            minimum_vote_duration_hours: 24,
            maximum_vote_duration_hours: 168, // 1 semaine
            quorum_percentage: 0.1, // 10% de participation minimum
            supermajority_threshold: 0.67, // 2/3 pour super-majorité
        }
    }
}

impl Blockchain {
    /// Crée une nouvelle blockchain avec le bloc genesis
    pub fn new() -> Self {
        let genesis_block = Block::genesis();
        
        Self {
            blocks: vec![genesis_block],
            accounts: HashMap::new(),
            laws: HashMap::new(),
            votes: HashMap::new(),
            proposals: HashMap::new(),
            proposal_supporters: HashMap::new(),
            verifications_by_account: HashMap::new(),
            verifications_by_id: HashMap::new(),
            identity_index: HashMap::new(),
            pending_transactions: Vec::new(),
            difficulty: 1,
            block_reward: 100,
        }
    }

    /// Reconstruit l'état (comptes, lois, votes, propositions, supporters) à partir des blocs
    /// en rejouant toutes les transactions. À utiliser après chargement des blocs depuis le stockage.
    pub fn rebuild_state_from_blocks(&mut self) -> Result<()> {
        // Réinitialiser l'état (conserver les blocs tels que chargés)
        self.accounts.clear();
        self.laws.clear();
        self.votes.clear();
        self.proposals.clear();
        self.proposal_supporters.clear();
        self.verifications_by_account.clear();
        self.verifications_by_id.clear();
        self.identity_index.clear();
        self.pending_transactions.clear();

        // Pour éviter les conflits d'emprunt, collecter toutes les transactions
        let all_txs: Vec<_> = self
            .blocks
            .iter()
            .flat_map(|b| b.transactions.clone())
            .collect();

        for tx in all_txs.iter() {
            // Appliquer chaque transaction pour reconstruire l'état
            self.apply_transaction(tx)?;
        }

        Ok(())
    }
    
    /// Obtient le dernier bloc
    pub fn last_block(&self) -> &Block {
        self.blocks.last().expect("La blockchain doit avoir au moins le bloc genesis")
    }
    
    /// Obtient la hauteur de la blockchain
    pub fn height(&self) -> u64 {
        self.blocks.len() as u64 - 1
    }
    
    /// Ajoute une transaction en attente
    pub fn add_pending_transaction(&mut self, transaction: Transaction) -> Result<()> {
        // Vérifier la validité de la transaction
        transaction.verify()
            .map_err(|e| BlockchainError::InvalidTransaction(e))?;
        
        // Vérifier que la transaction n'existe pas déjà
        if self.pending_transactions.iter().any(|tx| tx.id == transaction.id) {
            return Err(BlockchainError::InvalidTransaction(
                "Transaction déjà en attente".to_string()
            ));
        }
        
        self.pending_transactions.push(transaction);
        Ok(())
    }
    
    /// Mine un nouveau bloc avec les transactions en attente
    pub fn mine_block(&mut self, config: &BlockchainConfig) -> Result<Block> {
        let previous_hash = self.last_block().hash.clone();
        let block_number = self.height() + 1;
        
        // Sélectionner les transactions à inclure
        let transactions = self.select_transactions_for_block(config);
        
        if transactions.is_empty() {
            return Err(BlockchainError::InvalidTransaction(
                "Aucune transaction à miner".to_string()
            ));
        }
        
        // Créer le nouveau bloc
        let mut block = Block::new(previous_hash, transactions, block_number, self.difficulty);
        
        // Miner le bloc (proof-of-work)
        println!("🔨 Démarrage du minage du bloc #{} avec difficulté {}", block_number, self.difficulty);
        let start_time = std::time::Instant::now();
        
        block.mine(self.difficulty)
            .map_err(|e| BlockchainError::InvalidBlock(e))?;
        
        let mining_duration = start_time.elapsed();
        println!("✅ Bloc miné en {:?} avec nonce: {}", mining_duration, block.header.nonce);
        
        // Valider le bloc
        block.verify(Some(self.last_block()))
            .map_err(|e| BlockchainError::InvalidBlock(e))?;
        
        // Vérifier le proof-of-work
        if !block.verify_proof_of_work() {
            return Err(BlockchainError::InvalidBlock(
                "Proof-of-work invalide".to_string()
            ));
        }
        
        // Appliquer les changements d'état
        self.apply_block_transactions(&block)?;
        
        // Ajouter le bloc à la chaîne
        self.blocks.push(block.clone());
        
        // Nettoyer les transactions traitées
        self.cleanup_processed_transactions(&block);
        
        // Ajuster la difficulté si nécessaire
        self.adjust_difficulty(config);
        
        Ok(block)
    }
    
    /// Sélectionne les transactions pour un nouveau bloc
    fn select_transactions_for_block(&self, config: &BlockchainConfig) -> Vec<Transaction> {
        let mut selected = Vec::new();
        let mut total_size = 0;
        
        // Trier par frais (plus élevé en premier)
        let mut sorted_transactions = self.pending_transactions.clone();
        sorted_transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        
        for transaction in sorted_transactions {
            if selected.len() >= config.max_transactions_per_block {
                break;
            }
            
            let tx_size = transaction.estimated_size();
            if total_size + tx_size > config.max_block_size {
                break;
            }
            
            selected.push(transaction);
            total_size += tx_size;
        }
        
        selected
    }
    
    /// Applique les transactions d'un bloc à l'état
    fn apply_block_transactions(&mut self, block: &Block) -> Result<()> {
        for transaction in &block.transactions {
            self.apply_transaction(transaction)?;
        }
        Ok(())
    }
    
    /// Applique une transaction à l'état de la blockchain
    fn apply_transaction(&mut self, transaction: &Transaction) -> Result<()> {
        use crate::TransactionType;
        
        match &transaction.transaction_type {
            TransactionType::CreateAccount(account) => {
                if self.accounts.contains_key(&account.id) {
                    return Err(BlockchainError::InvalidTransaction(
                        "Compte déjà existant".to_string()
                    ));
                }
                // Unicité par clé publique: empêcher plusieurs comptes avec la même clé
                if self.accounts.values().any(|a| a.public_key == account.public_key) {
                    return Err(BlockchainError::InvalidTransaction(
                        "Clé publique déjà utilisée par un autre compte".to_string(),
                    ));
                }
                self.accounts.insert(account.id, account.clone());
            }
            
            TransactionType::UpdateAccount(account_id, updated_account) => {
                if !self.accounts.contains_key(account_id) {
                    return Err(BlockchainError::AccountNotFound(account_id.to_string()));
                }
                self.accounts.insert(*account_id, updated_account.clone());
            }
            
            TransactionType::CreateLaw(law) => {
                if self.laws.contains_key(&law.id) {
                    return Err(BlockchainError::InvalidTransaction(
                        "Loi déjà existante".to_string()
                    ));
                }
                self.laws.insert(law.id, law.clone());
            }
            
            TransactionType::UpdateLaw(law_id, updated_law) => {
                if !self.laws.contains_key(law_id) {
                    return Err(BlockchainError::LawNotFound(law_id.to_string()));
                }
                self.laws.insert(*law_id, updated_law.clone());
            }
            
            TransactionType::SubmitVote(vote) => {
                // Vérifier que la loi existe et que le vote est ouvert
                let law = self.laws.get(&vote.law_id)
                    .ok_or_else(|| BlockchainError::LawNotFound(vote.law_id.to_string()))?;
                
                if !law.is_voting_open() {
                    return Err(BlockchainError::InvalidVote(
                        "Le vote n'est pas ouvert".to_string()
                    ));
                }
                
                // Vérifier que le votant n'a pas déjà voté
                if let Some(existing_votes) = self.votes.get(&vote.law_id) {
                    if existing_votes.iter().any(|v| v.voter == vote.voter) {
                        return Err(BlockchainError::InvalidVote(
                            "Vote déjà enregistré".to_string()
                        ));
                    }
                }
                
                // Ajouter le vote
                self.votes.entry(vote.law_id).or_insert_with(Vec::new).push(vote.clone());
            }
            
            TransactionType::UpdateReputation(account_id, new_reputation) => {
                if let Some(account) = self.accounts.get_mut(account_id) {
                    account.reputation = *new_reputation;
                } else {
                    return Err(BlockchainError::AccountNotFound(account_id.to_string()));
                }
            }

            // ===== Propositions citoyennes =====
            TransactionType::CreateProposal(proposal) => {
                if self.proposals.contains_key(&proposal.id) {
                    return Err(BlockchainError::InvalidTransaction(
                        "Proposition déjà existante".to_string(),
                    ));
                }
                self.proposals.insert(proposal.id, proposal.clone());
                self.proposal_supporters.insert(proposal.id, HashSet::new());
            }

            TransactionType::SupportProposal { proposal_id, supporter } => {
                // La proposition doit exister
                if !self.proposals.contains_key(proposal_id) {
                    return Err(BlockchainError::InvalidTransaction(
                        "Proposition introuvable".to_string(),
                    ));
                }
                // Ne pas accepter de soutiens si expirée
                if let Some(p) = self.proposals.get(proposal_id) {
                    if p.expires_at <= Utc::now() {
                        return Err(BlockchainError::InvalidTransaction(
                            "Proposition expirée".to_string(),
                        ));
                    }
                }
                // Un même supporter (clé publique) ne peut soutenir qu'une fois
                let pk_hex = supporter.to_hex();
                let entry = self.proposal_supporters.entry(*proposal_id).or_default();
                if entry.contains(&pk_hex) {
                    return Err(BlockchainError::InvalidTransaction(
                        "Support déjà enregistré pour cet utilisateur".to_string(),
                    ));
                }
                entry.insert(pk_hex);
                // Mettre à jour le compteur pratique
                if let Some(p) = self.proposals.get_mut(proposal_id) {
                    p.supporters_count = p.supporters_count.saturating_add(1);
                    // Promotion automatique en loi à 100 soutiens
                    if p.supporters_count >= 100 {
                        // Créer une loi basique à partir de la proposition
                        let author = transaction.sender.clone();
                        let mut new_law = crate::Law::new(
                            p.title.clone(),
                            p.full_text.clone(),
                            p.description.clone(),
                            p.category.clone(),
                            author,
                            crate::law::LawChangeType::Creation,
                        );
                        new_law.status = crate::law::LawStatus::Active;
                        self.laws.insert(new_law.id, new_law);
                    }
                }
            }

            // ===== Vérification des comptes =====
            TransactionType::VerifyAccount(att) => {
                // Le compte doit exister
                let Some(account) = self.accounts.get_mut(&att.account_id) else {
                    return Err(BlockchainError::AccountNotFound(att.account_id.to_string()));
                };
                // Unicité: ce hash ne doit pas être pris par un autre compte
                if let Some(owner) = self.identity_index.get(&att.national_id_hash) {
                    if owner != &att.account_id {
                        return Err(BlockchainError::InvalidTransaction(
                            "Identité déjà liée à un autre compte".to_string(),
                        ));
                    }
                }
                // Vérifier expiration
                if let Some(exp) = att.expires_at {
                    if exp <= Utc::now() {
                        return Err(BlockchainError::InvalidTransaction(
                            "Attestation expirée".to_string(),
                        ));
                    }
                }
                // Pour une v1, on suppose la signature de l'issuer déjà validée à l'entrée
                account.metadata.verified = true;
                self.identity_index.insert(att.national_id_hash.clone(), att.account_id);
                self.verifications_by_account
                    .entry(att.account_id)
                    .or_default()
                    .push(att.clone());
                self.verifications_by_id.insert(att.id, att.clone());
            }

            TransactionType::RevokeVerification { attestation_id, reason: _ } => {
                // Trouver l'attestation par id
                let Some(att) = self.verifications_by_id.remove(attestation_id) else {
                    return Err(BlockchainError::InvalidTransaction(
                        "Attestation introuvable".to_string(),
                    ));
                };

                // Retirer des attestations du compte
                if let Some(list) = self.verifications_by_account.get_mut(&att.account_id) {
                    list.retain(|a| a.id != att.id);
                    // Si plus aucune attestation, marquer non vérifié
                    if list.is_empty() {
                        if let Some(acc) = self.accounts.get_mut(&att.account_id) {
                            acc.metadata.verified = false;
                        }
                    }
                }

                // Mettre à jour l'index d'identité uniquement si aucune autre attestation
                // avec le même hash n'existe pour ce compte
                let still_has_same_hash = self
                    .verifications_by_account
                    .get(&att.account_id)
                    .map(|list| list.iter().any(|a| a.national_id_hash == att.national_id_hash))
                    .unwrap_or(false);
                if !still_has_same_hash {
                    self.identity_index.remove(&att.national_id_hash);
                }
            }
        }
        
        Ok(())
    }
    
    /// Nettoie les transactions traitées
    fn cleanup_processed_transactions(&mut self, block: &Block) {
        let processed_ids: std::collections::HashSet<_> = 
            block.transactions.iter().map(|tx| tx.id).collect();
        
        self.pending_transactions.retain(|tx| !processed_ids.contains(&tx.id));
    }
    
    /// Obtient les votes pour une loi
    pub fn get_votes_for_law(&self, law_id: &uuid::Uuid) -> Vec<Vote> {
        self.votes.get(law_id).cloned().unwrap_or_default()
    }
    
    /// Calcule les résultats d'un vote
    pub fn calculate_vote_results(&self, law_id: &uuid::Uuid) -> Option<crate::VoteResults> {
        let votes = self.get_votes_for_law(law_id);
        if votes.is_empty() {
            return None;
        }
        
        let total_accounts = self.accounts.len() as u64;
        Some(crate::VoteResults::calculate_from_votes(*law_id, &votes, total_accounts))
    }
    
    /// Ajuste la difficulté basée sur le temps de bloc moyen
    fn adjust_difficulty(&mut self, config: &BlockchainConfig) {
        let blocks_len = self.blocks.len() as u64;
        
        // N'ajuster que tous les X blocs
        if blocks_len % config.difficulty_adjustment_interval != 0 || blocks_len < config.difficulty_adjustment_interval {
            return;
        }
        
        let adjustment_start = blocks_len - config.difficulty_adjustment_interval;
        let start_time = self.blocks[adjustment_start as usize].header.timestamp;
        let end_time = self.last_block().header.timestamp;
        
        let actual_time = (end_time - start_time).num_seconds() as u64;
        let expected_time = config.difficulty_adjustment_interval * config.target_block_time_seconds;
        
        let old_difficulty = self.difficulty;
        
        if actual_time < expected_time / 2 {
            // Trop rapide, augmenter la difficulté
            self.difficulty += 1;
        } else if actual_time > expected_time * 2 {
            // Trop lent, diminuer la difficulté (minimum 1)
            if self.difficulty > 1 {
                self.difficulty -= 1;
            }
        }
        
        if self.difficulty != old_difficulty {
            println!("🎯 Ajustement de difficulté: {} -> {} (temps: {}s vs {}s attendu)", 
                old_difficulty, self.difficulty, actual_time, expected_time);
        }
    }
    
    /// Obtient les statistiques de minage
    pub fn get_mining_stats(&self) -> MiningStats {
        if self.blocks.len() < 2 {
            return MiningStats::default();
        }
        
        let recent_blocks = self.blocks.iter().rev().take(10).collect::<Vec<_>>();
        let mut total_time = 0i64;
        let mut hash_rate_estimate = 0u64;
        
        for window in recent_blocks.windows(2) {
            let newer = window[0];
            let older = window[1];
            let time_diff = (newer.header.timestamp - older.header.timestamp).num_seconds();
            total_time += time_diff;
            
            // Estimation approximative du hash rate basée sur le nonce
            hash_rate_estimate += newer.header.nonce / (time_diff.max(1) as u64);
        }
        
        let average_block_time = if recent_blocks.len() > 1 {
            total_time / (recent_blocks.len() as i64 - 1)
        } else {
            0
        };
        
        MiningStats {
            difficulty: self.difficulty,
            average_block_time: average_block_time as u64,
            estimated_hash_rate: hash_rate_estimate / recent_blocks.len().max(1) as u64,
            total_blocks: self.blocks.len() as u64,
            pending_transactions: self.pending_transactions.len(),
        }
    }
    
    /// Vérifie la validité complète de la blockchain
    pub fn verify_chain(&self) -> Result<()> {
        if self.blocks.is_empty() {
            return Err(BlockchainError::InvalidBlock("Blockchain vide".to_string()));
        }
        
        // Vérifier le bloc genesis
        let genesis = &self.blocks[0];
        if genesis.header.block_number != 0 || !genesis.header.previous_hash.is_zero() {
            return Err(BlockchainError::InvalidBlock("Bloc genesis invalide".to_string()));
        }
        
        // Vérifier chaque bloc
        for i in 1..self.blocks.len() {
            let current_block = &self.blocks[i];
            let previous_block = &self.blocks[i - 1];
            
            current_block.verify(Some(previous_block))
                .map_err(|e| BlockchainError::InvalidBlock(e))?;
                
            // Vérifier le proof-of-work
            if !current_block.verify_proof_of_work() {
                return Err(BlockchainError::InvalidBlock(
                    format!("Proof-of-work invalide pour le bloc {}", i)
                ));
            }
        }
        
        Ok(())
    }
}

/// Statistiques de minage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningStats {
    pub difficulty: u32,
    pub average_block_time: u64,
    pub estimated_hash_rate: u64,
    pub total_blocks: u64,
    pub pending_transactions: usize,
}

impl Default for MiningStats {
    fn default() -> Self {
        Self {
            difficulty: 1,
            average_block_time: 0,
            estimated_hash_rate: 0,
            total_blocks: 0,
            pending_transactions: 0,
        }
    }
}


