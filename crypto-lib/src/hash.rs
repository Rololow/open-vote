use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::errors::{CryptoError, Result};

/// Hash SHA-256 pour l'intégrité des données
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Calcule le hash SHA-256 de données
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }

    /// Crée un hash à partir de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey("Le hash doit faire 32 bytes".to_string()));
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(bytes);
        Ok(Hash(hash))
    }

    /// Convertit le hash en bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Convertit en représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Crée à partir d'une chaîne hexadécimale
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| CryptoError::SerializationError(format!("Hex invalide: {}", e)))?;
        Self::from_bytes(&bytes)
    }

    /// Hash vide (zéros)
    pub fn zero() -> Self {
        Hash([0u8; 32])
    }

    /// Vérifie si le hash est vide
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }

    /// Hash de plusieurs éléments concaténés
    pub fn multi_hash(elements: &[&[u8]]) -> Self {
        let mut hasher = Sha256::new();
        for element in elements {
            hasher.update(element);
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}


