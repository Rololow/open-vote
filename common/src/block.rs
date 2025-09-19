use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crypto_lib::Hash;
use crate::Transaction;

/// En-tête d'un bloc dans la blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: DateTime<Utc>,
    pub difficulty: u32,
    pub nonce: u64,
    pub block_number: u64,
}

/// Bloc dans la blockchain e-gouvernement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub hash: Hash,
    pub validator: Option<String>, // Validateur du bloc (pour consensus)
}

impl Block {
    /// Crée le bloc genesis (premier bloc)
    pub fn genesis() -> Self {
        let header = BlockHeader {
            version: 1,
            previous_hash: Hash::zero(),
            merkle_root: Hash::zero(),
            timestamp: Utc::now(),
            difficulty: 1,
            nonce: 0,
            block_number: 0,
        };
        
        let hash = Self::calculate_hash(&header, &[]);
        
        Self {
            header,
            transactions: Vec::new(),
            hash,
            validator: None,
        }
    }
    
    /// Crée un nouveau bloc
    pub fn new(
        previous_hash: Hash,
        transactions: Vec<Transaction>,
        block_number: u64,
        difficulty: u32,
    ) -> Self {
        let merkle_root = Self::calculate_merkle_root(&transactions);
        let timestamp = Utc::now();
        
        let header = BlockHeader {
            version: 1,
            previous_hash,
            merkle_root,
            timestamp,
            difficulty,
            nonce: 0,
            block_number,
        };
        
        let hash = Self::calculate_hash(&header, &transactions);
        
        Self {
            header,
            transactions,
            hash,
            validator: None,
        }
    }
    
    /// Calcule le hash du bloc
    pub fn calculate_hash(header: &BlockHeader, transactions: &[Transaction]) -> Hash {
        let header_data = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            header.version,
            header.previous_hash.to_hex(),
            header.merkle_root.to_hex(),
            header.timestamp.timestamp(),
            header.difficulty,
            header.nonce,
            header.block_number
        );
        
        let tx_hashes: Vec<String> = transactions
            .iter()
            .map(|tx| tx.calculate_hash().to_hex())
            .collect();
        
        let combined_data = format!("{}:{}", header_data, tx_hashes.join(":"));
        Hash::new(combined_data.as_bytes())
    }
    
    /// Calcule la racine de Merkle des transactions
    fn calculate_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::zero();
        }
        
        let tx_hashes: Vec<Hash> = transactions
            .iter()
            .map(|tx| tx.calculate_hash())
            .collect();
        
        Self::build_merkle_tree(&tx_hashes)
    }
    
    /// Construit l'arbre de Merkle récursivement
    fn build_merkle_tree(hashes: &[Hash]) -> Hash {
        match hashes.len() {
            0 => Hash::zero(),
            1 => hashes[0].clone(),
            _ => {
                let mut next_level = Vec::new();
                
                // Traiter les paires de hashes
                for chunk in hashes.chunks(2) {
                    if chunk.len() == 2 {
                        let combined = Hash::multi_hash(&[
                            chunk[0].as_ref(),
                            chunk[1].as_ref(),
                        ]);
                        next_level.push(combined);
                    } else {
                        // Nombre impair, dupliquer le dernier élément
                        let combined = Hash::multi_hash(&[
                            chunk[0].as_ref(),
                            chunk[0].as_ref(),
                        ]);
                        next_level.push(combined);
                    }
                }
                
                Self::build_merkle_tree(&next_level)
            }
        }
    }
    
    /// Vérifie la validité du bloc
    pub fn verify(&self, previous_block: Option<&Block>) -> Result<(), String> {
        // Vérifier le hash du bloc
        let calculated_hash = Self::calculate_hash(&self.header, &self.transactions);
        if calculated_hash != self.hash {
            return Err("Hash du bloc invalide".to_string());
        }
        
        // Vérifier la racine de Merkle
        let calculated_merkle = Self::calculate_merkle_root(&self.transactions);
        if calculated_merkle != self.header.merkle_root {
            return Err("Racine de Merkle invalide".to_string());
        }
        
        // Vérifier le lien avec le bloc précédent
        if let Some(prev_block) = previous_block {
            if self.header.previous_hash != prev_block.hash {
                return Err("Hash du bloc précédent invalide".to_string());
            }
            
            if self.header.block_number != prev_block.header.block_number + 1 {
                return Err("Numéro de bloc invalide".to_string());
            }
            
            if self.header.timestamp <= prev_block.header.timestamp {
                return Err("Timestamp invalide".to_string());
            }
        } else if self.header.block_number != 0 {
            return Err("Bloc non-genesis sans prédécesseur".to_string());
        }
        
        // Vérifier toutes les transactions
        for transaction in &self.transactions {
            transaction.verify()
                .map_err(|e| format!("Transaction invalide: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Obtient le nombre total de transactions
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
    
    /// Obtient la taille estimée du bloc
    pub fn estimated_size(&self) -> usize {
        let header_size = 200; // estimation
        let tx_size: usize = self.transactions
            .iter()
            .map(|tx| tx.estimated_size())
            .sum();
        
        header_size + tx_size
    }
    
    /// Trouve une transaction par ID
    pub fn find_transaction(&self, transaction_id: &uuid::Uuid) -> Option<&Transaction> {
        self.transactions
            .iter()
            .find(|tx| tx.id == *transaction_id)
    }
    
    /// Mine le bloc avec proof-of-work
    pub fn mine(&mut self, difficulty: u32) -> Result<(), String> {
        let target = self.calculate_target(difficulty);
        let mut attempts = 0u64;
        
        loop {
            // Calculer le hash avec le nonce actuel
            let hash = Self::calculate_hash(&self.header, &self.transactions);
            
            // Vérifier si le hash respecte la difficulté
            if self.meets_difficulty(&hash, &target) {
                self.hash = hash;
                return Ok(());
            }
            
            // Incrémenter le nonce et réessayer
            self.header.nonce = self.header.nonce.wrapping_add(1);
            attempts += 1;
            
            // Loguer les progrès toutes les 100000 tentatives
            if attempts % 100_000 == 0 {
                println!("Mining en cours... {} tentatives", attempts);
            }
            
            // Mettre à jour le timestamp périodiquement
            if attempts % 1_000_000 == 0 {
                self.header.timestamp = Utc::now();
            }
        }
    }
    
    /// Calcule la cible pour la difficulté donnée
    fn calculate_target(&self, difficulty: u32) -> Vec<u8> {
        let leading_zeros = difficulty / 4;
        let remaining_bits = difficulty % 4;
        
        let mut target = vec![0u8; leading_zeros as usize];
        
        if remaining_bits > 0 {
            let mask = 0xFFu8 >> remaining_bits;
            target.push(mask);
        }
        
        // Compléter avec 0xFF pour le reste
        while target.len() < 32 {
            target.push(0xFF);
        }
        
        target
    }
    
    /// Vérifie si le hash respecte la difficulté
    fn meets_difficulty(&self, hash: &Hash, target: &[u8]) -> bool {
        let hash_bytes = hash.as_ref();
        
        for (i, (&hash_byte, &target_byte)) in hash_bytes.iter().zip(target.iter()).enumerate() {
            if i >= 32 { break; }
            
            if hash_byte < target_byte {
                return true;
            } else if hash_byte > target_byte {
                return false;
            }
        }
        
        false
    }
    
    /// Vérifie si le bloc a été correctement miné
    pub fn verify_proof_of_work(&self) -> bool {
        let target = self.calculate_target(self.header.difficulty);
        self.meets_difficulty(&self.hash, &target)
    }
}
