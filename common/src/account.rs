use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crypto_lib::{PublicKey, Hash};

/// Compte utilisateur dans le système e-gouvernement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub public_key: PublicKey,
    pub created_at: DateTime<Utc>,
    pub reputation: u64,
    pub is_active: bool,
    pub metadata: AccountMetadata,
}

/// Métadonnées du compte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_hash: Option<Hash>,
    pub verified: bool,
    pub specializations: Vec<String>, // Domaines d'expertise
}

impl Account {
    pub fn new(public_key: PublicKey) -> Self {
        Self {
            id: Uuid::new_v4(),
            public_key,
            created_at: Utc::now(),
            reputation: 0,
            is_active: true,
            metadata: AccountMetadata {
                display_name: None,
                bio: None,
                avatar_hash: None,
                verified: false,
                specializations: Vec::new(),
            },
        }
    }
    
    /// Calcule le score de réputation ajusté
    pub fn adjusted_reputation(&self) -> u64 {
        if !self.is_active {
            return 0;
        }
        
        let base_reputation = self.reputation;
        let verification_bonus = if self.metadata.verified { 100 } else { 0 };
        let specialization_bonus = self.metadata.specializations.len() as u64 * 10;
        
        base_reputation + verification_bonus + specialization_bonus
    }
    
    /// Vérifie si le compte peut voter sur un sujet donné
    pub fn can_vote_on_topic(&self, _topic: &str) -> bool {
        if !self.is_active {
            return false;
        }
        
        // Tous les comptes actifs peuvent voter
        // Les spécialisations peuvent donner un poids différent au vote
        true
    }
    
    /// Obtient le poids du vote pour un sujet donné
    pub fn vote_weight_for_topic(&self, topic: &str) -> f64 {
        let base_weight = 1.0;
        let reputation_multiplier = (self.adjusted_reputation() as f64).log10().max(1.0);
        
        // Bonus si spécialisé dans le domaine
        let specialization_bonus = if self.metadata.specializations
            .iter()
            .any(|spec| topic.to_lowercase().contains(&spec.to_lowercase())) {
            1.5
        } else {
            1.0
        };
        
        base_weight * reputation_multiplier * specialization_bonus
    }
}
