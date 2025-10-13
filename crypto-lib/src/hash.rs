use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use crate::errors::{CryptoError, Result};

/// SHA-256 hash for data integrity.
///
/// Small value type wrapping a 32-byte array with convenience helpers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Compute the SHA-256 hash of data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crypto_lib::Hash;
    /// let h = Hash::new(b"abc");
    /// assert!(!h.is_zero());
    /// ```
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Hash(hash)
    }

    /// Create a hash from bytes.
    ///
    /// # Errors
    /// Returns `CryptoError::InvalidKey` if the provided slice is not 32 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey("Le hash doit faire 32 bytes".to_string()));
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(bytes);
        Ok(Hash(hash))
    }

    /// Convert the hash to bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Convert to hexadecimal representation.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from a hexadecimal string.
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| CryptoError::SerializationError(format!("Hex invalide: {}", e)))?;
        Self::from_bytes(&bytes)
    }

    /// Zero hash (all zeros).
    pub fn zero() -> Self {
        Hash([0u8; 32])
    }

    /// Check if the hash is zero.
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }

    /// Hash multiple concatenated elements.
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


