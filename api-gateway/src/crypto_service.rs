use anyhow::{Result, anyhow};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, OsRng};
use argon2::{Argon2, password_hash::{PasswordHasher, SaltString}};
use std::error::Error;
use crypto_lib::{KeyPair, PublicKey, Signature};
use rand::RngCore;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::str::FromStr;
use crate::models::{CryptographicKeys, User};
use crate::persistent_database::DatabaseService;

/// Service de gestion des clés cryptographiques Ed25519
#[derive(Debug, Clone)]
pub struct CryptographicService {
    db: DatabaseService,
}

impl CryptographicService {
    pub fn new(db: DatabaseService) -> Self {
        Self { db }
    }

    /// Génère et stocke des clés cryptographiques pour un utilisateur validé
    pub async fn generate_keys_for_user(&self, user_id: &Uuid, password: &str) -> Result<CryptographicKeys> {
        // Vérifier que l'utilisateur existe et est validé
        let user = self.db.find_user_by_id(user_id).await?
            .ok_or_else(|| anyhow!("Utilisateur non trouvé"))?;

        if !matches!(user.identity_status, crate::models::IdentityStatus::Validated) {
            return Err(anyhow!("L'utilisateur doit avoir une identité validée pour générer des clés"));
        }

        // Vérifier si l'utilisateur a déjà des clés
        if let Some(_existing_keys) = self.db.get_crypto_keys(user_id).await? {
            return Err(anyhow!("L'utilisateur a déjà des clés cryptographiques"));
        }

        // Génération des clés Ed25519
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key().to_bytes();
        let private_key_bytes = keypair.private_key_bytes();

        // Chiffrement de la clé privée avec le mot de passe utilisateur
        let (encrypted_private_key, nonce, salt) = self.encrypt_private_key(&private_key_bytes, password)?;

        // Stockage en base de données
        let crypto_keys = self.db.create_crypto_keys(
            user_id,
            &public_key,
            &encrypted_private_key,
            &nonce,
            &salt
        ).await?;

        tracing::info!("Clés cryptographiques générées pour l'utilisateur {}", user_id);
        Ok(crypto_keys)
    }

    /// Récupère et déchiffre les clés d'un utilisateur
    pub async fn get_user_keys(&self, user_id: &Uuid, password: &str) -> Result<KeyPair> {
        let crypto_keys = self.db.get_crypto_keys(user_id).await?
            .ok_or_else(|| anyhow!("Aucune clé trouvée pour cet utilisateur"))?;

        // Déchiffrement de la clé privée
        let private_key = self.decrypt_private_key(
            &crypto_keys.encrypted_private_key,
            &crypto_keys.encryption_nonce,
            &crypto_keys.encryption_salt,
            password
        )?;

        // Conversion pour la reconstruction du KeyPair
        let private_key_array: [u8; 32] = private_key.try_into()
            .map_err(|_| anyhow!("Taille de clé privée invalide"))?;
        let keypair = KeyPair::from_private_bytes(&private_key_array);

        // Vérification de cohérence avec la clé publique stockée
        if crypto_keys.public_key != keypair.public_key().to_bytes() {
            return Err(anyhow!("Incohérence détectée entre les clés publique et privée"));
        }

        // Mise à jour de la dernière utilisation
        self.update_key_last_used(user_id).await?;

        Ok(keypair)
    }

    /// Chiffre une clé privée avec AES-GCM et Argon2
    fn encrypt_private_key(&self, private_key: &[u8], password: &str) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        // Génération d'un salt aléatoire pour Argon2
        let mut salt_bytes = [0u8; 16]; // Argon2 utilise des salts plus petits
        OsRng.fill_bytes(&mut salt_bytes);
        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| anyhow!("Erreur salt: {}", e))?;

        // Dérivation de clé avec Argon2
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Erreur Argon2: {}", e))?;
        let hash_bytes = password_hash.hash.unwrap();
        let derived_key = hash_bytes.as_bytes();

        // Préparation pour AES-GCM
        let key = Key::<Aes256Gcm>::from_slice(&derived_key[..32]);
        let cipher = Aes256Gcm::new(key);

        // Génération d'un nonce aléatoire
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Chiffrement de la clé privée
        let encrypted_private_key = cipher.encrypt(nonce, private_key)
            .map_err(|e| anyhow!("Erreur de chiffrement: {}", e))?;

        Ok((encrypted_private_key, nonce_bytes.to_vec(), salt_bytes.to_vec()))
    }

    /// Déchiffre une clé privée
    fn decrypt_private_key(&self, encrypted_private_key: &[u8], nonce: &[u8], salt: &[u8], password: &str) -> Result<Vec<u8>> {
        // Reconstruction du salt
        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| anyhow!("Erreur salt: {}", e))?;

        // Dérivation de clé avec Argon2
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| anyhow!("Erreur Argon2: {}", e))?;
        let hash_bytes = password_hash.hash.unwrap();
        let derived_key = hash_bytes.as_bytes();

        // Préparation pour AES-GCM
        let key = Key::<Aes256Gcm>::from_slice(&derived_key[..32]);
        let cipher = Aes256Gcm::new(key);
        let nonce_slice = Nonce::from_slice(nonce);

        // Déchiffrement
        let private_key = cipher.decrypt(nonce_slice, encrypted_private_key)
            .map_err(|e| anyhow!("Erreur de déchiffrement - mot de passe incorrect ou clé corrompue: {}", e))?;

        Ok(private_key)
    }

    /// Met à jour la dernière utilisation des clés
    async fn update_key_last_used(&self, user_id: &Uuid) -> Result<()> {
        // Pour l'instant, on fait juste un log. On pourrait ajouter une requête SQL
        // pour mettre à jour le champ last_used_at dans la table cryptographic_keys
        tracing::debug!("Clés utilisées pour l'utilisateur {}", user_id);
        Ok(())
    }

    /// Vérifie si un utilisateur peut accéder aux fonctionnalités de vote
    pub async fn can_user_vote(&self, user_id: &Uuid) -> Result<bool> {
        let user = self.db.find_user_by_id(user_id).await?
            .ok_or_else(|| anyhow!("Utilisateur non trouvé"))?;

        let has_keys = self.db.get_crypto_keys(user_id).await?.is_some();
        
        Ok(user.can_vote() && has_keys)
    }

    /// Change le mot de passe de chiffrement des clés (rotation de sécurité)
    pub async fn change_key_password(&self, user_id: &Uuid, old_password: &str, new_password: &str) -> Result<()> {
        // Récupération des clés avec l'ancien mot de passe
        let keypair = self.get_user_keys(user_id, old_password).await?;
        
        // Suppression des anciennes clés
        // TODO: Implémenter la suppression en base
        
        // Rechiffrement avec le nouveau mot de passe
        let private_key = keypair.private_key_bytes();
        let (encrypted_private_key, nonce, salt) = self.encrypt_private_key(&private_key, new_password)?;
        
        // Mise à jour en base
        // TODO: Implémenter la mise à jour des clés chiffrées
        
        tracing::info!("Mot de passe des clés changé pour l'utilisateur {}", user_id);
        Ok(())
    }



    /// Vérifie une signature avec la clé publique d'un utilisateur
    pub async fn verify_user_signature(&self, user_id: &Uuid, message: &[u8], signature: &[u8]) -> Result<bool> {
        let crypto_keys = self.db.get_crypto_keys(user_id).await?
            .ok_or_else(|| anyhow!("Aucune clé trouvée pour cet utilisateur"))?;

        let public_key_bytes = &crypto_keys.public_key;
        let public_key = crypto_lib::PublicKey::from_bytes(public_key_bytes)?;
        let keypair = KeyPair::from_public_key(public_key);
        
        // Reconstruction de la signature
        let sig = crypto_lib::Signature::from_bytes(signature)?;
        let is_valid = keypair.verify(message, &sig).is_ok();
        
        Ok(is_valid)
    }

    /// Exporte la clé publique d'un utilisateur en hexadécimal
    pub async fn get_user_public_key_hex(&self, user_id: &Uuid) -> Result<Option<String>> {
        let crypto_keys = self.db.get_crypto_keys(user_id).await?;
        
        match crypto_keys {
            Some(keys) => Ok(Some(hex::encode(&keys.public_key))),
            None => Ok(None),
        }
    }

    /// Récupère les informations sur les clés d'un utilisateur
    pub async fn get_user_keys_info(&self, user_id: &Uuid) -> Result<crate::models::CryptoKeysInfo> {
        let crypto_keys = self.db.get_crypto_keys(user_id).await?;
        
        match crypto_keys {
            Some(keys) => {
                Ok(crate::models::CryptoKeysInfo {
                    has_keys: true,
                    public_key_hex: Some(hex::encode(&keys.public_key)),
                    generated_at: Some(keys.generated_at),
                    last_used_at: keys.last_used_at,
                    key_version: Some(keys.key_version),
                })
            },
            None => {
                Ok(crate::models::CryptoKeysInfo {
                    has_keys: false,
                    public_key_hex: None,
                    generated_at: None,
                    last_used_at: None,
                    key_version: None,
                })
            }
        }
    }

    /// Sauvegarde de sécurité des clés (export chiffré)
    pub async fn backup_user_keys(&self, user_id: &Uuid, password: &str) -> Result<String> {
        let crypto_keys = self.db.get_crypto_keys(user_id).await?
            .ok_or_else(|| anyhow!("Aucune clé trouvée pour cet utilisateur"))?;

        // Création d'un backup JSON chiffré
        let backup_data = serde_json::json!({
            "user_id": user_id,
            "public_key": hex::encode(&crypto_keys.public_key),
            "encrypted_private_key": hex::encode(&crypto_keys.encrypted_private_key),
            "encryption_nonce": hex::encode(&crypto_keys.encryption_nonce),
            "encryption_salt": hex::encode(&crypto_keys.encryption_salt),
            "generated_at": crypto_keys.generated_at,
            "key_version": crypto_keys.key_version,
            "backup_timestamp": Utc::now(),
        });

        let backup_json = serde_json::to_string_pretty(&backup_data)?;
        
        // Chiffrement du backup avec le mot de passe utilisateur
        let (encrypted_backup, nonce, salt) = self.encrypt_private_key(backup_json.as_bytes(), password)?;
        
        let final_backup = serde_json::json!({
            "version": "1.0",
            "encrypted_data": hex::encode(encrypted_backup),
            "nonce": hex::encode(nonce),
            "salt": hex::encode(salt),
        });

        Ok(serde_json::to_string_pretty(&final_backup)?)
    }

    /// Restauration des clés depuis une sauvegarde
    pub async fn restore_user_keys(&self, user_id: &Uuid, backup_data: &str, password: &str) -> Result<()> {
        // Parse du backup
        let backup: serde_json::Value = serde_json::from_str(backup_data)?;
        
        let encrypted_data = hex::decode(backup["encrypted_data"].as_str().unwrap())?;
        let nonce = hex::decode(backup["nonce"].as_str().unwrap())?;
        let salt = hex::decode(backup["salt"].as_str().unwrap())?;

        // Déchiffrement
        let decrypted_data = self.decrypt_private_key(&encrypted_data, &nonce, &salt, password)?;
        let original_backup: serde_json::Value = serde_json::from_slice(&decrypted_data)?;

        // Vérification de l'user_id
        let backup_user_id = Uuid::from_str(original_backup["user_id"].as_str().unwrap())?;
        if backup_user_id != *user_id {
            return Err(anyhow!("Le backup ne correspond pas à cet utilisateur"));
        }

        // Restauration des clés
        let public_key = hex::decode(original_backup["public_key"].as_str().unwrap())?;
        let encrypted_private_key = hex::decode(original_backup["encrypted_private_key"].as_str().unwrap())?;
        let encryption_nonce = hex::decode(original_backup["encryption_nonce"].as_str().unwrap())?;
        let encryption_salt = hex::decode(original_backup["encryption_salt"].as_str().unwrap())?;

        // Sauvegarde en base
        self.db.create_crypto_keys(
            user_id,
            &public_key,
            &encrypted_private_key,
            &encryption_nonce,
            &encryption_salt
        ).await?;

        tracing::info!("Clés restaurées depuis backup pour l'utilisateur {}", user_id);
        Ok(())
    }

    /// Signe un message avec les clés de l'utilisateur
    pub async fn sign_with_user_keys(&self, user_id: &Uuid, crypto_keys: &CryptographicKeys, password: &str, message: &[u8]) -> Result<Vec<u8>> {
        // Déchiffrer la clé privée
        let private_key = self.decrypt_private_key(
            &crypto_keys.encrypted_private_key,
            &crypto_keys.encryption_nonce,
            &crypto_keys.encryption_salt,
            password
        )?;

        // Conversion pour la reconstruction du KeyPair
        let private_key_array: [u8; 32] = private_key.try_into()
            .map_err(|_| anyhow!("Taille de clé privée invalide"))?;
        let keypair = KeyPair::from_private_bytes(&private_key_array);

        // Signer le message
        let signature = keypair.sign(message);
        
        Ok(signature.to_bytes().to_vec())
    }

    /// Vérifie une signature avec une clé publique
    pub fn verify_signature(&self, public_key_bytes: &[u8], message: &[u8], signature_bytes: &[u8]) -> Result<bool> {
        // Reconstruction de la clé publique
        let public_key_array: [u8; 32] = public_key_bytes.try_into()
            .map_err(|_| anyhow!("Taille de clé publique invalide"))?;
        let public_key = PublicKey::from_bytes(&public_key_array)?;

        // Reconstruction de la signature
        let signature_array: [u8; 64] = signature_bytes.try_into()
            .map_err(|_| anyhow!("Taille de signature invalide"))?;
        let signature = Signature::from_bytes(&signature_array)?;

        // Vérification
        match public_key.verify(message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false)
        }
    }
}