use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use crate::errors::{CryptoError, Result};

/// Clé publique Ed25519
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey);

/// Clé privée Ed25519 avec effacement sécurisé
#[derive(Debug, Clone)]
pub struct PrivateKey(SigningKey);

/// Paire de clés cryptographiques
#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public_key: PublicKey,
    private_key: PrivateKey,
}

impl PublicKey {
    /// Crée une clé publique à partir de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let verifying_key = VerifyingKey::try_from(bytes)
            .map_err(|e| CryptoError::InvalidKey(format!("Clé publique invalide: {}", e)))?;
        Ok(PublicKey(verifying_key))
    }

    /// Convertit la clé publique en bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Convertit en représentation hexadécimale
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    /// Crée à partir d'une chaîne hexadécimale
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
    /// Vérifie une signature
    pub fn verify(&self, message: &[u8], signature: &crate::signature::Signature) -> Result<()> {
        self.0.verify(message, &signature.0)
            .map_err(|e| CryptoError::VerificationError(format!("Échec vérification: {}", e)))
    }
}

impl PrivateKey {
    /// Crée une clé privée à partir de bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKey("La clé privée doit faire 32 bytes".to_string()));
        }
        
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(bytes);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        Ok(PrivateKey(signing_key))
    }

    /// Convertit la clé privée en bytes (attention: sensible!)
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Signe un message
    pub fn sign(&self, message: &[u8]) -> crate::signature::Signature {
        let signature = self.0.sign(message);
        crate::signature::Signature(signature)
    }

    /// Obtient la clé publique correspondante
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }
}

impl KeyPair {
    /// Génère une nouvelle paire de clés aléatoire
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

    /// Crée une paire de clés à partir d'une graine
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let private_key = PrivateKey(signing_key);
        let public_key = private_key.public_key();
        
        KeyPair {
            public_key,
            private_key,
        }
    }

    /// Accès à la clé publique
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Signe un message avec la clé privée
    pub fn sign(&self, message: &[u8]) -> crate::signature::Signature {
        self.private_key.sign(message)
    }

    /// Vérifie une signature avec la clé publique
    pub fn verify(&self, message: &[u8], signature: &crate::signature::Signature) -> Result<()> {
        self.public_key.verify(message, signature)
    }

    /// Accès sécurisé à la clé privée (pour chiffrement)
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key.to_bytes()
    }

    /// Crée une paire de clés à partir de bytes de clé privée
    pub fn from_private_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let private_key = PrivateKey(signing_key);
        let public_key = private_key.public_key();
        
        KeyPair {
            public_key,
            private_key,
        }
    }

    /// Crée une paire de clés à partir de clé publique uniquement (pour vérification)
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


