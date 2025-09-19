use ed25519_dalek::Signature as Ed25519Signature;
use serde::{Deserialize, Serialize};
use crate::errors::{CryptoError, Result};

/// Wrapper pour les signatures Ed25519
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub(crate) Ed25519Signature);

impl Signature {
    /// Crée une signature à partir de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 64 {
            return Err(CryptoError::InvalidKey("La signature doit faire 64 bytes".to_string()));
        }
        
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(bytes);
        
        let signature = Ed25519Signature::from_bytes(&sig_bytes);
        Ok(Signature(signature))
    }

    /// Convertit la signature en bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    /// Convertit en représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    /// Crée à partir d'une chaîne hexadécimale
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| CryptoError::SerializationError(format!("Hex invalide: {}", e)))?;
        Self::from_bytes(&bytes)
    }
}

// Implémentations manuelles de Serialize/Deserialize pour Signature
impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.to_bytes();
        serializer.serialize_str(&hex::encode(bytes))
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        Self::from_hex(&hex_str).map_err(serde::de::Error::custom)
    }
}


