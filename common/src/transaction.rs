use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crypto_lib::{Hash, PublicKey, Signature};
use crate::{Vote, Law, Account, proposal::Proposal, VerifiedAttestation};
#[cfg(feature = "identity")]
use crate::identity::zkp_prelude::AnonymousActionPayload;

/// Types de transactions dans le système e-gouvernement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    CreateAccount(Account),
    UpdateAccount(Uuid, Account),
    CreateLaw(Law),
    UpdateLaw(Uuid, Law),
    SubmitVote(Vote),
    UpdateReputation(Uuid, u64),
    // Propositions citoyennes
    CreateProposal(Proposal),
    /// Ajout d'un soutien (like/signature) à une proposition par une clé publique
    SupportProposal {
        proposal_id: Uuid,
        supporter: PublicKey,
    },
    /// Vérifier un compte avec une attestation sans PII
    VerifyAccount(VerifiedAttestation),
    /// Révoquer une attestation de vérification
    RevokeVerification { attestation_id: Uuid, reason: Option<String> },
    // ===== Anonymous actions (Phase 3)
    #[cfg(feature = "identity")]
    AnonymousVote { law_id: Uuid, proof: AnonymousActionPayload },
    #[cfg(feature = "identity")]
    AnonymousSupport { proposal_id: Uuid, proof: AnonymousActionPayload },
    // ===== Phase 5: Enhanced transaction types
    /// Identity validation event (emitted after verification)
    IdentityValidated {
        identity_hash: String,
        validator: PublicKey,
        timestamp: DateTime<Utc>,
    },
    /// Proposal created event (enriched metadata)
    ProposalCreated {
        proposal_id: Uuid,
        author: PublicKey,
        title: String,
        category: String,
    },
    /// Support added to proposal (for tracking)
    SupportAdded {
        proposal_id: Uuid,
        supporter: PublicKey,
        support_count: u32,
    },
    /// Law promoted from proposal (automatic promotion)
    LawPromoted {
        proposal_id: Uuid,
        law_id: Uuid,
        promoted_by: PublicKey,
        support_count: u32,
    },
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
    /// Référence optionnelle à un engagement d'identité (commitment hash hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_ref: Option<String>,
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
            identity_ref: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tx() -> Transaction {
        let kp = crypto_lib::KeyPair::generate();
        let ttype = TransactionType::UpdateReputation(Uuid::new_v4(), 1);
        let mut tx = Transaction::new(ttype, kp.public_key().clone(), kp.sign(b"temp"), 1, 0);
        // Re-signer with proper message format
        let sign_msg = format!(
            "TRANSACTION:{}:{}:{}",
            tx.id,
            tx.timestamp,
            tx.data_hash.to_hex()
        );
        tx.signature = kp.sign(sign_msg.as_bytes());
        tx
    }

    #[test]
    fn serialize_without_identity_ref_omits_field() {
        let tx = sample_tx();
        let json = serde_json::to_string(&tx).unwrap();
        assert!(!json.contains("identity_ref"), "identity_ref should be omitted when None");
    }

    #[test]
    fn serialize_with_identity_ref_includes_field() {
        let mut tx = sample_tx();
        tx.identity_ref = Some("deadbeef".into());
        let json = serde_json::to_string(&tx).unwrap();
        assert!(json.contains("identity_ref"), "identity_ref should be present when Some");
        let back: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.identity_ref.as_deref(), Some("deadbeef"));
    }
}
