//! common — shared types used across workspace crates.
//!
//! This crate exposes domain models and shared types for the
//! e-government blockchain project, such as blocks, transactions,
//! laws, votes, accounts and API types. Optional DID/VC identity
//! functionality is available behind the `identity` feature.
//!
//! Public modules re-exported for easy consumption by other crates.
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
pub mod api_types;
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
pub use api_types::*;

// Types for authentication and role-based permissions
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Copy)]
/// Role of a user in the system.
///
/// Used for simple role-based permission checks.
pub enum UserRole {
    /// Regular citizen user.
    Citizen,
    /// Government user with elevated privileges.
    Government,
}

impl UserRole {
    /// Check whether this role has the permissions of the `required` role.
    pub fn has_permission(&self, required: &UserRole) -> bool {
        match (self, required) {
            // Égalité stricte entre Citoyen et Gouvernement (pas de hiérarchie)
            (UserRole::Citizen, UserRole::Citizen) => true,
            (UserRole::Citizen, UserRole::Government) => true,
            (UserRole::Government, UserRole::Citizen) => true,
            (UserRole::Government, UserRole::Government) => true,
        }
    }
    
    /// Return a numeric priority for the role (higher = more permissions).
    pub fn priority(&self) -> u8 {
        match self { UserRole::Government | UserRole::Citizen => 10 }
    }

    /// Return default permissions associated with the role.
    pub fn default_permissions(&self) -> Vec<String> {
        match self {
            UserRole::Citizen => vec!["create_law".to_string(), "vote_on_law".to_string()],
            UserRole::Government => vec!["create_law".to_string(), "vote_on_law".to_string()],
        }
    }
}
