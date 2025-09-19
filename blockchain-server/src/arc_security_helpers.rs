// Helpers pour Arc<SecurityManager>
use std::sync::Arc;
use crate::security::{SecurityManager, SecurityLevel, Block};
use anyhow::Result;

/// Implémentation de méthodes pour Arc<SecurityManager> qui délègue aux méthodes de SecurityManager
impl Arc<SecurityManager> {
    /// Délègue à SecurityManager::get_security_status
    pub fn get_security_status(&self, level: &SecurityLevel) -> String {
        self.as_ref().get_security_status(level)
    }

    /// Délègue à SecurityManager::mine_block_with_difficulty
    pub fn mine_block_with_difficulty(&self, difficulty: u64) -> u64 {
        self.as_ref().mine_block_with_difficulty(difficulty)
    }

    /// Délègue à SecurityManager::validate_transaction_security
    pub fn validate_transaction_security(&self, tx_data: &str) -> bool {
        self.as_ref().validate_transaction_security(tx_data)
    }

    /// Délègue à SecurityManager::get_detected_attacks_count
    pub fn get_detected_attacks_count(&self) -> u64 {
        self.as_ref().get_detected_attacks_count()
    }
}