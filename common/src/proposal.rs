use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use chrono::Duration;

/// Statut d'une proposition citoyenne (workflow simplifié)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    #[serde(rename = "Brouillon")] 
    Draft,
    #[serde(rename = "Collecte signatures")]
    CollectingSignatures,
    #[serde(rename = "En révision")]
    InReview,
    #[serde(rename = "Approuvée")]
    Approved,
    #[serde(rename = "Rejetée")]
    Rejected,
}

impl ToString for ProposalStatus {
    fn to_string(&self) -> String {
        match self {
            ProposalStatus::Draft => "Brouillon".to_string(),
            ProposalStatus::CollectingSignatures => "Collecte signatures".to_string(),
            ProposalStatus::InReview => "En révision".to_string(),
            ProposalStatus::Approved => "Approuvée".to_string(),
            ProposalStatus::Rejected => "Rejetée".to_string(),
        }
    }
}

/// Modèle de proposition citoyenne (état on-chain simplifié)
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
    pub created_at: DateTime<Utc>,
    /// Date d'expiration de la collecte de soutiens
    #[serde(default = "default_expires_at")]
    pub expires_at: DateTime<Utc>,
    pub status: ProposalStatus,
    /// Compteur pratique; l'ensemble des supporters est maintenu séparément dans la blockchain
    #[serde(default)]
    pub supporters_count: u32,
}

impl Proposal {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

fn default_expires_at() -> DateTime<Utc> {
    Utc::now() + Duration::days(30)
}
