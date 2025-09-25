use serde::{Deserialize, Serialize};
use std::fmt;

/// ArticleNumber supports up to 3 levels of depth (e.g., 1, 1.2, 1.2.3)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArticleNumber {
    parts: Vec<u32>,
}

impl ArticleNumber {
    /// Parse a number like "1", "1.2", or "1.2.3" with max depth 3.
    pub fn parse<S: AsRef<str>>(s: S) -> Result<Self, String> {
        let s = s.as_ref().trim();
        if s.is_empty() {
            return Err("Numéro d'article vide".into());
        }
        let parts: Result<Vec<u32>, _> = s
            .split('.')
            .map(|p| {
                if p.is_empty() { return Err("Segment vide".to_string()); }
                let n: u32 = p
                    .parse()
                    .map_err(|_| format!("Segment non numérique: {}", p))?;
                if n == 0 { return Err("Les numéros doivent commencer à 1".to_string()); }
                Ok(n)
            })
            .collect();

        let parts = parts.map_err(|e| e.to_string())?;
        if parts.is_empty() || parts.len() > 3 {
            return Err("Profondeur maximale 3 (ex: 1.2.3)".into());
        }
        Ok(Self { parts })
    }

    /// Create from parts, enforcing max depth 3 and positive integers.
    pub fn from_parts(parts: &[u32]) -> Result<Self, String> {
        if parts.is_empty() || parts.len() > 3 { return Err("Profondeur maximale 3".into()); }
        if parts.iter().any(|&p| p == 0) { return Err("Les numéros doivent commencer à 1".into()); }
        Ok(Self { parts: parts.to_vec() })
    }

    pub fn depth(&self) -> usize { self.parts.len() }
    pub fn parts(&self) -> &[u32] { &self.parts }
}

impl fmt::Display for ArticleNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(".");
        write!(f, "{}", s)
    }
}

/// Minimal article representation (content may be stored on-chain or in DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub law_id: uuid::Uuid,
    pub number: ArticleNumber,
    pub title: Option<String>,
    pub text: String,
    pub version: u32,
    pub author: String, // hex public key or user id
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Workflow status for two-step proposals (draft -> review/collect -> approved/rejected)
    pub status: ArticleProposalStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArticleProposalStatus {
    Draft,
    CollectingSignatures,
    InReview,
    Approved,
    Rejected,
}
