use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use crate::errors::{CryptoError, Result};

/// Ed25519 public key.
///
/// Lightweight wrapper around `ed25519_dalek::VerifyingKey` with
/// (de)serialization helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey);

/// Ed25519 private key (handle bytes with care).
///
/// Wrapper around `ed25519_dalek::SigningKey`. Exposes methods to
/// create from bytes and sign messages.
#[derive(Debug, Clone)]
pub struct PrivateKey(SigningKey);

/// Cryptographic key pair.
///
/// Contains the public and private key. The `from_public_key` constructor
/// creates a pair usable for verification only.
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// The public key component of this key pair.
    pub public_key: PublicKey,
    private_key: PrivateKey,
}

impl PublicKey {
    /// Create a public key from bytes.
    ///
    /// # Errors
    /// Returns `CryptoError::InvalidKey` if the bytes cannot be parsed.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let verifying_key = VerifyingKey::try_from(bytes)
            .map_err(|e| CryptoError::InvalidKey(format!("Clé publique invalide: {}", e)))?;
        Ok(PublicKey(verifying_key))
    }

    /// Convert the public key to bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Convert to hexadecimal representation.
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    /// Create from a hexadecimal string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crypto_lib::PublicKey;
    /// assert!(PublicKey::from_hex("00").is_err());
    /// ```
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| CryptoError::InvalidKey(format!("Hex invalide: {}", e)))?;
        Self::from_bytes(&bytes)
    }
}

// Implémentations manuelles de Serialize/Deserialize pour PublicKey
impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.to_bytes();
        serializer.serialize_str(&hex::encode(bytes))
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        Self::from_hex(&hex_str).map_err(serde::de::Error::custom)
    }
}

impl PublicKey {
    /// Verify a signature.
    pub fn verify(&self, message: &[u8], signature: &crate::signature::Signature) -> Result<()> {
        self.0.verify(message, &signature.0)
            .map_err(|e| CryptoError::VerificationError(format!("Échec vérification: {}", e)))
    }
}

impl PrivateKey {
    /// Create a private key from bytes.
    ///
    /// # Errors
    /// Returns `CryptoError::InvalidKey` if length != 32.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey("La clé privée doit faire 32 bytes".to_string()));
        }
        
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        Ok(PrivateKey(signing_key))
    }

    /// Convert the private key to bytes (sensitive!).
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> crate::signature::Signature {
        let signature = self.0.sign(message);
        crate::signature::Signature(signature)
    }

    /// Get the corresponding public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }
}

impl KeyPair {
    /// Generate a new random key pair.
    pub fn generate() -> Self {
        let _rng = OsRng;
        let signing_key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
        let private_key = PrivateKey(signing_key);
        let public_key = private_key.public_key();
        
        KeyPair {
            public_key,
            private_key,
        }
    }

    /// Create a key pair from a seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let private_key = PrivateKey(signing_key);
        let public_key = private_key.public_key();
        
        KeyPair {
            public_key,
            private_key,
        }
    }

    /// Access the public key.
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Sign a message with the private key.
    pub fn sign(&self, message: &[u8]) -> crate::signature::Signature {
        self.private_key.sign(message)
    }

    /// Verify a signature with the public key.
    pub fn verify(&self, message: &[u8], signature: &crate::signature::Signature) -> Result<()> {
        self.public_key.verify(message, signature)
    }

    /// Secure access to the private key bytes (for encryption).
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key.to_bytes()
    }

    /// Create a key pair from private key bytes.
    pub fn from_private_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let private_key = PrivateKey(signing_key);
        let public_key = private_key.public_key();
        
        KeyPair {
            public_key,
            private_key,
        }
    }

    /// Create a key pair from a public key only (for verification).
    pub fn from_public_key(public_key: PublicKey) -> Self {
        // Note: Cette paire ne peut pas signer, seulement vérifier
        let dummy_signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let private_key = PrivateKey(dummy_signing_key);
        
        KeyPair {
            public_key,
            private_key,
        }
    }
}


