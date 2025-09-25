pub mod blockchain;
pub mod transaction;
pub mod block;
pub mod law;
pub mod vote;
pub mod article;
pub mod account;
pub mod errors;
pub mod law_category;
pub mod proposal;
pub use blockchain::VerifiedAttestation;

// Module identité (DID / VC) sous feature flag pour limiter les dépendances.
// Pour l'activer : cargo build -p common --features identity
#[cfg(feature = "identity")]
pub mod identity;
#[cfg(feature = "identity")]
pub use identity::*;

pub use blockchain::*;
pub use transaction::*;
pub use block::*;
pub use law::*;
pub use vote::*;
pub use account::*;
pub use errors::*;
pub use law_category::LawCategory;

// Types pour l'authentification
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Copy)]
pub enum UserRole {
    Citizen,
    Government,
}

impl UserRole {
    /// Vérifie si ce rôle a les permissions d'un autre rôle
    pub fn has_permission(&self, required: &UserRole) -> bool {
        match (self, required) {
            // Égalité stricte entre Citoyen et Gouvernement (pas de hiérarchie)
            (UserRole::Citizen, UserRole::Citizen) => true,
            (UserRole::Citizen, UserRole::Government) => true,
            (UserRole::Government, UserRole::Citizen) => true,
            (UserRole::Government, UserRole::Government) => true,
        }
    }
    
    /// Obtient la priorité du rôle (plus élevé = plus de permissions)
    pub fn priority(&self) -> u8 {
        match self { UserRole::Government | UserRole::Citizen => 10 }
    }
    
    /// Obtient les permissions par défaut pour un rôle
    pub fn default_permissions(&self) -> Vec<String> {
        match self {
            UserRole::Citizen => vec!["create_law".to_string(), "vote_on_law".to_string()],
            UserRole::Government => vec!["create_law".to_string(), "vote_on_law".to_string()],
        }
    }
}
