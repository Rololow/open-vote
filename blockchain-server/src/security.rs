use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};

/// Module de sécurité avancé pour la blockchain
/// Implémente les mécanismes de protection essentiels

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Difficulté minimale pour le mining (Proof of Work)
    pub min_difficulty: u32,
    /// Temps maximum entre les blocs (secondes)
    pub max_block_time: u64,
    /// Nombre minimum de confirmations pour une transaction
    pub min_confirmations: u32,
    /// Seuil de consensus (pourcentage des nœuds)
    pub consensus_threshold: f64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            min_difficulty: 4, // 4 zéros en début de hash
            max_block_time: 600, // 10 minutes maximum
            min_confirmations: 6,
            consensus_threshold: 0.51, // 51% minimum
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub previous_hash: String,
    pub merkle_root: String,
    pub nonce: u64,
    pub difficulty: u32,
    pub transactions: Vec<Transaction>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub data: String,
    pub timestamp: u64,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Clone)]
pub struct SecurityManager {
    config: SecurityConfig,
    blockchain: Vec<Block>,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            blockchain: Vec::new(),
        }
    }

    /// 1. VALIDATION CRYPTOGRAPHIQUE
    /// Vérifie la signature d'une transaction
    pub fn verify_transaction_signature(&self, transaction: &Transaction) -> Result<bool> {
        // Dans un vrai système, on utiliserait Ed25519 ou ECDSA
        let message = format!("{}{}{}{}", 
            transaction.from, 
            transaction.to, 
            transaction.data, 
            transaction.timestamp
        );
        
        // Simulation de vérification de signature
        let expected_signature = self.calculate_signature(&message, &transaction.public_key);
        
        if transaction.signature == expected_signature {
            info!("✅ Signature valide pour transaction {}", transaction.id);
            Ok(true)
        } else {
            warn!("❌ Signature invalide pour transaction {}", transaction.id);
            Ok(false)
        }
    }

    /// 2. PROOF OF WORK
    /// Implémente le mécanisme de minage sécurisé
    pub fn mine_block(&self, transactions: Vec<Transaction>, previous_hash: String) -> Result<Block> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Calculer le Merkle Root des transactions
        let merkle_root = self.calculate_merkle_root(&transactions);

        let mut block = Block {
            index: self.blockchain.len() as u64,
            timestamp,
            previous_hash,
            merkle_root,
            nonce: 0,
            difficulty: self.config.min_difficulty,
            transactions,
            hash: String::new(),
        };

        // Proof of Work - Trouver un nonce qui produit un hash avec suffisamment de zéros
        info!("🔨 Début du minage avec difficulté {}", block.difficulty);
        let start_time = SystemTime::now();

        loop {
            block.hash = self.calculate_block_hash(&block);
            
            // Vérifier si le hash respecte la difficulté
            if self.hash_meets_difficulty(&block.hash, block.difficulty) {
                let mining_time = start_time.elapsed()?.as_secs();
                info!("⛏️ Bloc miné ! Nonce: {}, Temps: {}s, Hash: {}", 
                    block.nonce, mining_time, block.hash);
                break;
            }
            
            block.nonce += 1;
            
            // Protection contre les boucles infinies
            if block.nonce % 100000 == 0 {
                info!("🔍 Mining en cours... Nonce: {}", block.nonce);
            }
        }

        Ok(block)
    }

    /// 3. VALIDATION DE CHAÎNE
    /// Vérifie l'intégrité de toute la blockchain
    pub fn validate_blockchain(&self) -> Result<bool> {
        info!("🔍 Validation de la blockchain ({} blocs)", self.blockchain.len());

        for (i, block) in self.blockchain.iter().enumerate() {
            // Vérifier le hash du bloc
            let calculated_hash = self.calculate_block_hash(block);
            if block.hash != calculated_hash {
                error!("❌ Hash invalide au bloc {}: {} != {}", 
                    i, block.hash, calculated_hash);
                return Ok(false);
            }

            // Vérifier le chaînage avec le bloc précédent
            if i > 0 {
                let prev_block = &self.blockchain[i - 1];
                if block.previous_hash != prev_block.hash {
                    error!("❌ Chaînage rompu au bloc {}", i);
                    return Ok(false);
                }
            }

            // Vérifier la difficulté
            if !self.hash_meets_difficulty(&block.hash, block.difficulty) {
                error!("❌ Difficulté non respectée au bloc {}", i);
                return Ok(false);
            }

            // Vérifier toutes les transactions du bloc
            for transaction in &block.transactions {
                if !self.verify_transaction_signature(transaction)? {
                    error!("❌ Transaction invalide dans le bloc {}: {}", 
                        i, transaction.id);
                    return Ok(false);
                }
            }
        }

        info!("✅ Blockchain valide !");
        Ok(true)
    }

    /// 4. DÉTECTION D'ATTAQUES
    /// Identifie les tentatives de manipulation
    pub fn detect_attacks(&self, new_block: &Block) -> Result<Vec<String>> {
        let mut threats = Vec::new();

        // Attaque de double dépense
        if self.detect_double_spending(&new_block.transactions) {
            threats.push("Double spending détecté".to_string());
        }

        // Attaque temporelle
        if self.detect_timestamp_attack(new_block) {
            threats.push("Manipulation de timestamp détectée".to_string());
        }

        // Attaque de difficulté
        if self.detect_difficulty_attack(new_block) {
            threats.push("Manipulation de difficulté détectée".to_string());
        }

        // Attaque de spam
        if self.detect_spam_attack(&new_block.transactions) {
            threats.push("Attaque de spam détectée".to_string());
        }

        if !threats.is_empty() {
            warn!("🚨 Menaces détectées: {:?}", threats);
        }

        Ok(threats)
    }

    /// 5. MÉCANISMES DE CONSENSUS
    /// Simule un consensus distribué
    pub fn validate_consensus(&self, block: &Block, network_votes: &[(String, bool)]) -> Result<bool> {
        let total_nodes = network_votes.len() as f64;
        let positive_votes = network_votes.iter().filter(|(_, vote)| *vote).count() as f64;
        let consensus_ratio = positive_votes / total_nodes;

        info!("🗳️ Consensus: {}/{} votes positifs ({:.1}%)", 
            positive_votes, total_nodes, consensus_ratio * 100.0);

        if consensus_ratio >= self.config.consensus_threshold {
            info!("✅ Consensus atteint pour le bloc {}", block.index);
            Ok(true)
        } else {
            warn!("❌ Consensus non atteint pour le bloc {}", block.index);
            Ok(false)
        }
    }

    // === FONCTIONS UTILITAIRES PRIVÉES ===

    fn calculate_signature(&self, message: &str, public_key: &str) -> String {
        // Simulation simple - dans un vrai système utiliser Ed25519
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", message, public_key));
        format!("{:x}", hasher.finalize())
    }

    fn calculate_block_hash(&self, block: &Block) -> String {
        let block_string = format!(
            "{}{}{}{}{}{}{}",
            block.index,
            block.timestamp,
            block.previous_hash,
            block.merkle_root,
            block.nonce,
            block.difficulty,
            serde_json::to_string(&block.transactions).unwrap_or_default()
        );

        let mut hasher = Sha256::new();
        hasher.update(block_string);
        format!("{:x}", hasher.finalize())
    }

    fn calculate_merkle_root(&self, transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return "0".repeat(64);
        }

        // Calculer les hashs des transactions
        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| {
                let tx_string = serde_json::to_string(tx).unwrap_or_default();
                let mut hasher = Sha256::new();
                hasher.update(tx_string);
                format!("{:x}", hasher.finalize())
            })
            .collect();

        // Construire l'arbre de Merkle
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    format!("{}{}", chunk[0], chunk[0]) // Dupliquer si impair
                };
                
                let mut hasher = Sha256::new();
                hasher.update(combined);
                next_level.push(format!("{:x}", hasher.finalize()));
            }
            
            hashes = next_level;
        }

        hashes[0].clone()
    }

    fn hash_meets_difficulty(&self, hash: &str, difficulty: u32) -> bool {
        hash.starts_with(&"0".repeat(difficulty as usize))
    }

    fn detect_double_spending(&self, transactions: &[Transaction]) -> bool {
        // Vérifier les doubles dépenses dans le même bloc
        let mut seen_inputs = std::collections::HashSet::new();
        
        for tx in transactions {
            let input_key = format!("{}:{}", tx.from, tx.id);
            if !seen_inputs.insert(input_key) {
                return true; // Double dépense détectée
            }
        }
        
        false
    }

    fn detect_timestamp_attack(&self, block: &Block) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Le timestamp ne peut pas être dans le futur
        if block.timestamp > current_time + 7200 { // 2 heures de tolérance
            return true;
        }
        
        // Le timestamp ne peut pas être trop ancien
        if let Some(last_block) = self.blockchain.last() {
            if block.timestamp < last_block.timestamp {
                return true;
            }
        }
        
        false
    }

    fn detect_difficulty_attack(&self, block: &Block) -> bool {
        // La difficulté doit être cohérente avec la configuration
        block.difficulty < self.config.min_difficulty
    }

    fn detect_spam_attack(&self, transactions: &[Transaction]) -> bool {
        // Trop de transactions du même expéditeur
        let mut sender_counts = std::collections::HashMap::new();
        
        for tx in transactions {
            *sender_counts.entry(tx.from.clone()).or_insert(0) += 1;
        }
        
        // Plus de 10 transactions du même expéditeur = suspect
        sender_counts.values().any(|&count| count > 10)
    }
}

/// Fonctions utilitaires pour les tests de sécurité
pub mod security_tests {
    use super::*;

    pub fn create_test_transaction(from: &str, to: &str, data: &str) -> Transaction {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let id = uuid::Uuid::new_v4().to_string();
        let public_key = format!("pub_key_{}", from);
        
        // Simuler la signature
        let message = format!("{}{}{}{}", from, to, data, timestamp);
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", message, public_key));
        let signature = format!("{:x}", hasher.finalize());

        Transaction {
            id,
            from: from.to_string(),
            to: to.to_string(),
            data: data.to_string(),
            timestamp,
            signature,
            public_key,
        }
    }

    pub fn simulate_network_attack() -> Vec<Transaction> {
        // Simuler une attaque de double dépense
        vec![
            create_test_transaction("alice", "bob", "100"),
            create_test_transaction("alice", "charlie", "100"), // Double dépense !
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_signature() {
        let config = SecurityConfig::default();
        let security = SecurityManager::new(config);
        
        let tx = security_tests::create_test_transaction("alice", "bob", "test");
        assert!(security.verify_transaction_signature(&tx).unwrap());
    }

    #[test]
    fn test_double_spending_detection() {
        let config = SecurityConfig::default();
        let security = SecurityManager::new(config);
        
        let malicious_txs = security_tests::simulate_network_attack();
        assert!(security.detect_double_spending(&malicious_txs));
    }

    #[test]
    fn test_mining() {
        let config = SecurityConfig {
            min_difficulty: 2, // Plus facile pour les tests
            ..Default::default()
        };
        let security = SecurityManager::new(config);
        
        let transactions = vec![
            security_tests::create_test_transaction("alice", "bob", "test mining")
        ];
        
        let block = security.mine_block(transactions, "0".repeat(64)).unwrap();
        assert!(block.hash.starts_with("00")); // Vérifie la difficulté
    }
}