use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crypto_lib::{Hash, PublicKey, Signature};
use crate::{Vote, Law, Account};

/// Types de transactions dans le système e-gouvernement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    CreateAccount(Account),
    UpdateAccount(Uuid, Account),
    CreateLaw(Law),
    UpdateLaw(Uuid, Law),
    SubmitVote(Vote),
    UpdateReputation(Uuid, u64),
}

/// Transaction dans la blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub transaction_type: TransactionType,
    pub sender: PublicKey,
    pub timestamp: DateTime<Utc>,
    pub signature: Signature,
    pub nonce: u64,
    pub fee: u64,
    pub data_hash: Hash,
}

impl Transaction {
    pub fn new(
        transaction_type: TransactionType,
        sender: PublicKey,
        signature: Signature,
        nonce: u64,
        fee: u64,
    ) -> Self {
        let serialized = serde_json::to_string(&transaction_type)
            .unwrap_or_else(|_| "invalid_transaction".to_string());
        let data_hash = Hash::new(serialized.as_bytes());
        
        Self {
            id: Uuid::new_v4(),
            transaction_type,
            sender,
            timestamp: Utc::now(),
            signature,
            nonce,
            fee,
            data_hash,
        }
    }
    
    /// Calcule le hash de la transaction
    pub fn calculate_hash(&self) -> Hash {
        let tx_data = format!(
            "{}:{}:{}:{}:{}:{}",
            self.id,
            self.sender.to_hex(),
            self.timestamp.timestamp(),
            self.nonce,
            self.fee,
            self.data_hash.to_hex()
        );
        Hash::new(tx_data.as_bytes())
    }
    
    /// Vérifie la validité de la transaction
    pub fn verify(&self) -> Result<(), String> {
        // Vérifier l'intégrité des données
        let serialized = serde_json::to_string(&self.transaction_type)
            .map_err(|e| format!("Erreur sérialisation: {}", e))?;
        let calculated_hash = Hash::new(serialized.as_bytes());
        
        if calculated_hash != self.data_hash {
            return Err("Hash des données invalide".to_string());
        }
        
        // Vérifier la signature
        let message = format!(
            "TRANSACTION:{}:{}:{}",
            self.id,
            self.timestamp,
            self.data_hash.to_hex()
        );
        
        self.sender.verify(message.as_bytes(), &self.signature)
            .map_err(|e| format!("Signature invalide: {:?}", e))?;
        
        Ok(())
    }
    
    /// Détermine si la transaction affecte un compte spécifique
    pub fn affects_account(&self, account_id: &Uuid) -> bool {
        match &self.transaction_type {
            TransactionType::CreateAccount(account) => &account.id == account_id,
            TransactionType::UpdateAccount(id, _) => id == account_id,
            TransactionType::UpdateReputation(id, _) => id == account_id,
            TransactionType::SubmitVote(vote) => {
                // Le vote affecte l'account qui vote
                vote.voter == self.sender
            },
            _ => false,
        }
    }
    
    /// Obtient la taille estimée en bytes
    pub fn estimated_size(&self) -> usize {
        serde_json::to_string(self)
            .map(|s| s.len())
            .unwrap_or(1000) // estimation par défaut
    }
}
