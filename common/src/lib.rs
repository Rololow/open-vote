pub mod blockchain;
pub mod transaction;
pub mod block;
pub mod law;
pub mod vote;
pub mod account;
pub mod errors;

pub use blockchain::*;
pub use transaction::*;
pub use block::*;
pub use law::*;
pub use vote::*;
pub use account::*;
pub use errors::*;

// Types pour l'authentification
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Copy)]
pub enum UserRole {
    Citizen,
    Government,
    Administrator,
}

impl UserRole {
    /// Vérifie si ce rôle a les permissions d'un autre rôle
    pub fn has_permission(&self, required: &UserRole) -> bool {
        match (self, required) {
            (UserRole::Administrator, _) => true,
            (UserRole::Government, UserRole::Citizen) => true,
            (UserRole::Government, UserRole::Government) => true,
            (UserRole::Citizen, UserRole::Citizen) => true,
            _ => false,
        }
    }
    
    /// Obtient la priorité du rôle (plus élevé = plus de permissions)
    pub fn priority(&self) -> u8 {
        match self {
            UserRole::Administrator => 100,
            UserRole::Government => 80,
            UserRole::Citizen => 20,
        }
    }
    
    /// Obtient les permissions par défaut pour un rôle
    pub fn default_permissions(&self) -> Vec<String> {
        match self {
            UserRole::Citizen => vec![
                "create_law".to_string(),
                "vote_on_law".to_string(),
            ],
            UserRole::Government => vec![
                "create_law".to_string(),
                "vote_on_law".to_string(),
                "modify_law".to_string(),
            ],
            UserRole::Administrator => vec![
                "create_law".to_string(),
                "vote_on_law".to_string(),
                "modify_law".to_string(),
                "delete_law".to_string(),
                "view_audit_log".to_string(),
                "manage_users".to_string(),
                "moderate_content".to_string(),
                "access_admin_panel".to_string(),
                "export_data".to_string(),
                "system_maintenance".to_string(),
            ],
        }
    }
}
