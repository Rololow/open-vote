use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::fmt;
use crypto_lib::{Hash, PublicKey};
use crate::law_category::LawCategory;

/// Types de modifications sur les lois
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LawChangeType {
    Creation,      // Création d'une nouvelle loi
    Amendment,     // Modification d'une loi existante
    Repeal,        // Abrogation d'une loi
    Suspension,    // Suspension temporaire
}

/// Statut d'une loi
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LawStatus {
    Draft,         // Brouillon
    InReview,      // En cours de révision
    Voting,        // En cours de vote
    Approved,      // Approuvée
    Rejected,      // Rejetée
    Active,        // Active (en vigueur)
    Suspended,     // Suspendue
    Repealed,      // Abrogée
}

impl fmt::Display for LawStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LawStatus::Draft => "Draft",
            LawStatus::InReview => "InReview",
            LawStatus::Voting => "Voting",
            LawStatus::Approved => "Approved",
            LawStatus::Rejected => "Rejected",
            LawStatus::Active => "Active",
            LawStatus::Suspended => "Suspended",
            LawStatus::Repealed => "Repealed",
        };
        write!(f, "{}", s)
    }
}

/// Une loi dans le système
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Law {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub category: String,
    pub tags: Vec<String>,
    
    // Métadonnées
    pub status: LawStatus,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub author: PublicKey,
    
    // Versioning
    pub version: u32,
    pub parent_law_id: Option<Uuid>, // Pour les modifications
    pub change_type: LawChangeType,
    
    // Hash du contenu pour intégrité
    pub content_hash: Hash,
    
    // Voting
    pub voting_started_at: Option<DateTime<Utc>>,
    pub voting_ends_at: Option<DateTime<Utc>>,
    pub requires_supermajority: bool, // 2/3 au lieu de majorité simple
    
    // Références
    pub references: Vec<Uuid>, // Autres lois référencées
    pub supersedes: Vec<Uuid>, // Lois remplacées par celle-ci
}

impl Law {
    pub fn new(
        title: String,
        content: String,
        summary: String,
        category: String,
        author: PublicKey,
        change_type: LawChangeType,
    ) -> Self {
        let content_hash = Hash::new(content.as_bytes());
        
        Self {
            id: Uuid::new_v4(),
            title,
            content,
            summary,
            category,
            tags: Vec::new(),
            status: LawStatus::Draft,
            created_at: Utc::now(),
            last_modified: Utc::now(),
            author,
            version: 1,
            parent_law_id: None,
            change_type,
            content_hash,
            voting_started_at: None,
            voting_ends_at: None,
            requires_supermajority: false,
            references: Vec::new(),
            supersedes: Vec::new(),
        }
    }
    
    /// Crée une amendement à une loi existante
    pub fn create_amendment(
        parent_law: &Law,
        new_content: String,
        new_summary: String,
        author: PublicKey,
    ) -> Self {
        let content_hash = Hash::new(new_content.as_bytes());
        
        Self {
            id: Uuid::new_v4(),
            title: format!("Amendement à: {}", parent_law.title),
            content: new_content,
            summary: new_summary,
            category: parent_law.category.clone(),
            tags: parent_law.tags.clone(),
            status: LawStatus::Draft,
            created_at: Utc::now(),
            last_modified: Utc::now(),
            author,
            version: parent_law.version + 1,
            parent_law_id: Some(parent_law.id),
            change_type: LawChangeType::Amendment,
            content_hash,
            voting_started_at: None,
            voting_ends_at: None,
            requires_supermajority: parent_law.requires_supermajority,
            references: parent_law.references.clone(),
            supersedes: vec![parent_law.id],
        }
    }
    
    /// Démarre le processus de vote
    pub fn start_voting(&mut self, duration_hours: u32) -> Result<(), String> {
        if self.status != LawStatus::InReview {
            return Err("La loi doit être en révision pour commencer le vote".to_string());
        }
        
        self.status = LawStatus::Voting;
        self.voting_started_at = Some(Utc::now());
        self.voting_ends_at = Some(Utc::now() + chrono::Duration::hours(duration_hours as i64));
        
        Ok(())
    }
    
    /// Vérifie si le vote est encore ouvert
    pub fn is_voting_open(&self) -> bool {
        match (self.status.clone(), self.voting_ends_at) {
            (LawStatus::Voting, Some(end_time)) => Utc::now() < end_time,
            _ => false,
        }
    }
    
    /// Met à jour le contenu et recalcule le hash
    pub fn update_content(&mut self, new_content: String) {
        self.content = new_content;
        self.content_hash = Hash::new(self.content.as_bytes());
        self.last_modified = Utc::now();
    }
    
    /// Vérifie l'intégrité du contenu
    pub fn verify_content_integrity(&self) -> bool {
        let calculated_hash = Hash::new(self.content.as_bytes());
        calculated_hash == self.content_hash
    }

    /// Retourne la catégorie sous forme d'énum pour un usage type-safe côté UI
    pub fn category_enum(&self) -> LawCategory {
        LawCategory::from_str(&self.category)
    }
}
