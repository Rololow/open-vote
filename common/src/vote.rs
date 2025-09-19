use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crypto_lib::{PublicKey, Signature, Hash};

/// Types de vote possibles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteType {
    For,         // Pour
    Against,     // Contre  
    Abstain,     // Abstention
}

/// Un vote individuel sur une loi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub id: Uuid,
    pub law_id: Uuid,
    pub voter: PublicKey,
    pub vote_type: VoteType,
    pub weight: f64,      // Poids du vote basé sur réputation/expertise
    pub timestamp: DateTime<Utc>,
    pub signature: Signature, // Signature cryptographique du vote
    pub comment: Option<String>, // Commentaire optionnel
    pub vote_hash: Hash,  // Hash pour intégrité
}

/// Résultats agrégés d'un vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResults {
    pub law_id: Uuid,
    pub total_votes: u64,
    pub votes_for: u64,
    pub votes_against: u64,
    pub abstentions: u64,
    
    // Poids pondérés
    pub weighted_for: f64,
    pub weighted_against: f64,
    pub weighted_abstentions: f64,
    pub total_weight: f64,
    
    // Statistiques
    pub participation_rate: f64,
    pub approval_rate: f64,
    pub weighted_approval_rate: f64,
    
    pub calculated_at: DateTime<Utc>,
}

impl Vote {
    pub fn new(
        law_id: Uuid,
        voter: PublicKey,
        vote_type: VoteType,
        weight: f64,
        comment: Option<String>,
        signature: Signature,
    ) -> Self {
        let vote_data = format!("{:?}:{:?}:{:?}:{}", law_id, voter.to_hex(), vote_type, weight);
        let vote_hash = Hash::new(vote_data.as_bytes());
        
        Self {
            id: Uuid::new_v4(),
            law_id,
            voter,
            vote_type,
            weight,
            timestamp: Utc::now(),
            signature,
            comment,
            vote_hash,
        }
    }
    
    /// Vérifie l'intégrité cryptographique du vote
    pub fn verify_integrity(&self) -> Result<(), String> {
        let vote_data = format!("{:?}:{:?}:{:?}:{}", 
                               self.law_id, self.voter.to_hex(), self.vote_type, self.weight);
        let calculated_hash = Hash::new(vote_data.as_bytes());
        
        if calculated_hash != self.vote_hash {
            return Err("Hash du vote invalide".to_string());
        }
        
        // Vérifier la signature cryptographique
        let message = format!("VOTE:{}:{}:{:?}", self.law_id, self.timestamp, self.vote_type);
        self.voter.verify(message.as_bytes(), &self.signature)
            .map_err(|e| format!("Signature du vote invalide: {:?}", e))?;
        
        Ok(())
    }
}

impl VoteResults {
    pub fn new(law_id: Uuid) -> Self {
        Self {
            law_id,
            total_votes: 0,
            votes_for: 0,
            votes_against: 0,
            abstentions: 0,
            weighted_for: 0.0,
            weighted_against: 0.0,
            weighted_abstentions: 0.0,
            total_weight: 0.0,
            participation_rate: 0.0,
            approval_rate: 0.0,
            weighted_approval_rate: 0.0,
            calculated_at: Utc::now(),
        }
    }
    
    /// Calcule les résultats à partir d'une liste de votes
    pub fn calculate_from_votes(law_id: Uuid, votes: &[Vote], total_eligible_voters: u64) -> Self {
        let mut results = Self::new(law_id);
        results.total_votes = votes.len() as u64;
        
        for vote in votes {
            match vote.vote_type {
                VoteType::For => {
                    results.votes_for += 1;
                    results.weighted_for += vote.weight;
                }
                VoteType::Against => {
                    results.votes_against += 1;
                    results.weighted_against += vote.weight;
                }
                VoteType::Abstain => {
                    results.abstentions += 1;
                    results.weighted_abstentions += vote.weight;
                }
            }
            results.total_weight += vote.weight;
        }
        
        // Calcul des taux
        results.participation_rate = if total_eligible_voters > 0 {
            results.total_votes as f64 / total_eligible_voters as f64
        } else {
            0.0
        };
        
        results.approval_rate = if results.total_votes > 0 {
            results.votes_for as f64 / (results.votes_for + results.votes_against) as f64
        } else {
            0.0
        };
        
        results.weighted_approval_rate = if results.weighted_for + results.weighted_against > 0.0 {
            results.weighted_for / (results.weighted_for + results.weighted_against)
        } else {
            0.0
        };
        
        results.calculated_at = Utc::now();
        results
    }
    
    /// Détermine si la loi est approuvée (majorité simple)
    pub fn is_approved_simple_majority(&self) -> bool {
        self.weighted_approval_rate > 0.5
    }
    
    /// Détermine si la loi est approuvée (super-majorité 2/3)
    pub fn is_approved_super_majority(&self) -> bool {
        self.weighted_approval_rate >= 2.0/3.0
    }
    
    /// Vérifie si le quorum minimum est atteint
    pub fn meets_quorum(&self, minimum_participation: f64) -> bool {
        self.participation_rate >= minimum_participation
    }
}
