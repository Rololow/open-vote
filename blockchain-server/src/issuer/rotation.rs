use std::{path::PathBuf, fs, collections::HashMap};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::{info, warn};

/// Configuration for issuer key rotation (Phase 6.2)
/// Supports multiple verification keys for graceful rotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    /// Current active key version (used for signing new credentials)
    pub active_version: u32,
    /// Map of version -> verification key info
    pub verification_keys: HashMap<u32, VerificationKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationKeyInfo {
    /// Public key in hex format
    pub public_key_hex: String,
    /// DID derived from this public key
    pub did: String,
    /// When this key was activated
    pub activated_at: String,
    /// Optional: when this key was retired (if set, no new credentials signed)
    pub retired_at: Option<String>,
    /// Optional: when this key should be completely removed from trust
    pub revoked_at: Option<String>,
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            active_version: 1,
            verification_keys: HashMap::new(),
        }
    }
}

impl KeyRotationConfig {
    /// Load rotation config from file, or create new default
    pub fn load_or_create(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("Reading rotation config: {}", path.display()))?;
            let config: Self = serde_json::from_str(&raw)?;
            info!("✅ Loaded key rotation config: {} VKs, active version {}", 
                  config.verification_keys.len(), config.active_version);
            Ok(config)
        } else {
            info!("Creating new key rotation config at {}", path.display());
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    /// Save rotation config to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, json)?;
        info!("💾 Saved key rotation config to {}", path.display());
        Ok(())
    }

    /// Register a new verification key (when rotating)
    pub fn add_verification_key(
        &mut self,
        version: u32,
        public_key_hex: String,
        did: String,
    ) -> Result<()> {
        let info = VerificationKeyInfo {
            public_key_hex,
            did,
            activated_at: chrono::Utc::now().to_rfc3339(),
            retired_at: None,
            revoked_at: None,
        };
        
        self.verification_keys.insert(version, info);
        info!("✅ Added verification key version {}", version);
        Ok(())
    }

    /// Promote a new key version to active (rotation step)
    pub fn rotate_to_version(&mut self, new_version: u32) -> Result<()> {
        if !self.verification_keys.contains_key(&new_version) {
            anyhow::bail!("Cannot rotate to version {}: key not registered", new_version);
        }
        
        // Retire the old active version
        if let Some(old_vk) = self.verification_keys.get_mut(&self.active_version) {
            if old_vk.retired_at.is_none() {
                old_vk.retired_at = Some(chrono::Utc::now().to_rfc3339());
                info!("🔄 Retired version {}", self.active_version);
            }
        }
        
        self.active_version = new_version;
        info!("✅ Rotated to active version {}", new_version);
        Ok(())
    }

    /// Mark a key version as revoked (compromise detected)
    pub fn revoke_version(&mut self, version: u32, reason: &str) -> Result<()> {
        if let Some(vk) = self.verification_keys.get_mut(&version) {
            vk.revoked_at = Some(chrono::Utc::now().to_rfc3339());
            warn!("⚠️ REVOKED version {}: {}", version, reason);
            Ok(())
        } else {
            anyhow::bail!("Version {} not found", version)
        }
    }

    /// Check if a key version is trusted (not revoked)
    pub fn is_version_trusted(&self, version: u32) -> bool {
        if let Some(vk) = self.verification_keys.get(&version) {
            vk.revoked_at.is_none()
        } else {
            false
        }
    }

    /// Get all trusted verification keys (for credential verification)
    pub fn get_trusted_vks(&self) -> Vec<(u32, &VerificationKeyInfo)> {
        self.verification_keys
            .iter()
            .filter(|(_, vk)| vk.revoked_at.is_none())
            .map(|(v, vk)| (*v, vk))
            .collect()
    }

    /// Get the active verification key info
    pub fn get_active_vk(&self) -> Option<&VerificationKeyInfo> {
        self.verification_keys.get(&self.active_version)
    }

    /// Get a specific version's info
    pub fn get_vk(&self, version: u32) -> Option<&VerificationKeyInfo> {
        self.verification_keys.get(&version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_key_rotation_lifecycle() {
        let tmp_dir = env::temp_dir();
        let config_path = tmp_dir.join("rotation_test.json");
        
        // Clean up before test
        let _ = fs::remove_file(&config_path);
        
        // Create config
        let mut config = KeyRotationConfig::load_or_create(&config_path).unwrap();
        
        // Add version 1
        config.add_verification_key(
            1,
            "aabbcc".to_string(),
            "did:key:z1".to_string(),
        ).unwrap();
        config.active_version = 1;
        
        // Add version 2
        config.add_verification_key(
            2,
            "ddeeff".to_string(),
            "did:key:z2".to_string(),
        ).unwrap();
        
        // Rotate to version 2
        config.rotate_to_version(2).unwrap();
        assert_eq!(config.active_version, 2);
        
        // Version 1 should be retired but trusted
        assert!(config.is_version_trusted(1));
        let vk1 = config.get_vk(1).unwrap();
        assert!(vk1.retired_at.is_some());
        
        // Revoke version 1
        config.revoke_version(1, "test compromise").unwrap();
        assert!(!config.is_version_trusted(1));
        
        // Save and reload
        config.save(&config_path).unwrap();
        let reloaded = KeyRotationConfig::load_or_create(&config_path).unwrap();
        assert_eq!(reloaded.active_version, 2);
        assert!(!reloaded.is_version_trusted(1));
        assert!(reloaded.is_version_trusted(2));
        
        // Cleanup
        let _ = fs::remove_file(&config_path);
    }
}
