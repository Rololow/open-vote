use sqlx::{SqlitePool, Row};
use anyhow::{Result, anyhow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::{Aead, generic_array::GenericArray};
use rand::{RngCore, CryptoRng};

use crate::models::{
    User, IdentityValidation, CryptographicKeys, UserSession, IdentityStatus,
    RegisterRequest, LoginRequest, SubmitIdentityRequest
};
use common::UserRole;

/// Service de gestion de la base de données utilisateurs
#[derive(Debug, Clone)]
pub struct DatabaseService {
    pool: SqlitePool,
}

impl DatabaseService {
    /// Initialise la base de données et exécute les migrations
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        
        // Exécuter les migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }

    /// Crée un nouvel utilisateur avec hash Argon2 du mot de passe
    pub async fn create_user(&self, request: &RegisterRequest) -> Result<User> {
        // Vérifier si l'email existe déjà
        let existing_user = sqlx::query("SELECT COUNT(*) as count FROM users WHERE email = ?")
            .bind(&request.email)
            .fetch_one(&self.pool)
            .await?;
        
        if existing_user.get::<i64, _>("count") > 0 {
            return Err(anyhow!("Un utilisateur avec cet email existe déjà"));
        }

        // Hasher le mot de passe avec Argon2
        let password_hash = self.hash_password(&request.password)?;
        
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        // Insérer l'utilisateur
        sqlx::query(r#"
            INSERT INTO users (id, email, name, password_hash, role, identity_status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(user_id)
        .bind(&request.email)
        .bind(&request.name)
        .bind(&password_hash)
        .bind("citizen") // UserRole par défaut
        .bind("pending") // IdentityStatus par défaut
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // Récupérer l'utilisateur créé
        self.get_user_by_id(user_id).await
    }

    /// Authentifie un utilisateur et vérifie le mot de passe
    pub async fn authenticate_user(&self, request: &LoginRequest) -> Result<User> {
        let user_row = sqlx::query(r#"
            SELECT id, email, name, password_hash, role, identity_status, 
                   created_at, updated_at, identity_validation_id, crypto_keys_id
            FROM users WHERE email = ?
        "#)
        .bind(&request.email)
        .fetch_optional(&self.pool)
        .await?;

        let user_row = user_row.ok_or_else(|| anyhow!("Utilisateur non trouvé"))?;
        
        let password_hash: String = user_row.get("password_hash");
        
        // Vérifier le mot de passe avec Argon2
        if !self.verify_password(&request.password, &password_hash)? {
            return Err(anyhow!("Mot de passe incorrect"));
        }

        // Construire l'objet User
        Ok(User {
            id: user_row.get("id"),
            email: user_row.get("email"),
            name: user_row.get("name"),
            password_hash,
            role: self.parse_user_role(user_row.get("role"))?,
            identity_status: self.parse_identity_status(user_row.get("identity_status"))?,
            created_at: user_row.get("created_at"),
            updated_at: user_row.get("updated_at"),
            identity_validation_id: user_row.get("identity_validation_id"),
            crypto_keys_id: user_row.get("crypto_keys_id"),
        })
    }

    /// Récupère un utilisateur par ID
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User> {
        let user_row = sqlx::query(r#"
            SELECT id, email, name, password_hash, role, identity_status, 
                   created_at, updated_at, identity_validation_id, crypto_keys_id
            FROM users WHERE id = ?
        "#)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let user_row = user_row.ok_or_else(|| anyhow!("Utilisateur non trouvé"))?;
        
        Ok(User {
            id: user_row.get("id"),
            email: user_row.get("email"),
            name: user_row.get("name"),
            password_hash: user_row.get("password_hash"),
            role: self.parse_user_role(user_row.get("role"))?,
            identity_status: self.parse_identity_status(user_row.get("identity_status"))?,
            created_at: user_row.get("created_at"),
            updated_at: user_row.get("updated_at"),
            identity_validation_id: user_row.get("identity_validation_id"),
            crypto_keys_id: user_row.get("crypto_keys_id"),
        })
    }

    /// Soumet des documents pour validation d'identité
    pub async fn submit_identity_validation(&self, user_id: Uuid, request: &SubmitIdentityRequest) -> Result<IdentityValidation> {
        let validation_id = Uuid::new_v4();
        let now = Utc::now();

        // Créer le dossier de validation
        sqlx::query(r#"
            INSERT INTO identity_validations (
                id, user_id, document_type, document_number, document_name, 
                document_firstname, document_birth_date, status, submitted_at, 
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(validation_id)
        .bind(user_id)
        .bind(&request.document_type)
        .bind(&request.document_number)
        .bind(&request.document_name)
        .bind(&request.document_firstname)
        .bind(request.document_birth_date)
        .bind("submitted")
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // Mettre à jour l'utilisateur
        sqlx::query(r#"
            UPDATE users 
            SET identity_validation_id = ?, identity_status = ?, updated_at = ?
            WHERE id = ?
        "#)
        .bind(validation_id)
        .bind("submitted")
        .bind(now)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // TODO: Sauvegarder le fichier chiffré si fourni
        if let Some(_file_base64) = &request.document_file_base64 {
            // Logique de sauvegarde sécurisée à implémenter
        }

        self.get_identity_validation(validation_id).await
    }

    /// Génère et stocke les clés cryptographiques pour un utilisateur validé
    pub async fn generate_crypto_keys(&self, user_id: Uuid, password: &str) -> Result<CryptographicKeys> {
        // Vérifier que l'utilisateur est validé
        let user = self.get_user_by_id(user_id).await?;
        if !matches!(user.identity_status, IdentityStatus::Validated) {
            return Err(anyhow!("L'utilisateur doit être validé pour générer des clés"));
        }

        // Générer une paire de clés Ed25519
        let keypair = crypto_lib::KeyPair::generate();
        let public_key = keypair.public_key().to_bytes().to_vec();
        let private_key = keypair.private_key_bytes().to_vec();

        // Générer un salt et dériver une clé de chiffrement
        let mut salt = [0u8; 32];
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut salt);
        rand::thread_rng().fill_bytes(&mut nonce_bytes);

        let encryption_key = self.derive_key_from_password(password, &salt)?;
        let cipher = Aes256Gcm::new(&encryption_key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Chiffrer la clé privée
        let encrypted_private_key = cipher.encrypt(nonce, private_key.as_ref())
            .map_err(|e| anyhow!("Erreur de chiffrement: {}", e))?;

        let keys_id = Uuid::new_v4();
        let now = Utc::now();

        // Stocker les clés en base
        sqlx::query(r#"
            INSERT INTO cryptographic_keys (
                id, user_id, public_key, encrypted_private_key, encryption_nonce,
                encryption_salt, generated_at, key_version
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(keys_id)
        .bind(user_id)
        .bind(&public_key)
        .bind(&encrypted_private_key)
        .bind(&nonce_bytes[..])
        .bind(&salt[..])
        .bind(now)
        .bind(1) // Version 1
        .execute(&self.pool)
        .await?;

        // Mettre à jour l'utilisateur
        sqlx::query("UPDATE users SET crypto_keys_id = ?, updated_at = ? WHERE id = ?")
            .bind(keys_id)
            .bind(now)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        self.get_crypto_keys(keys_id).await
    }

    /// Récupère les clés cryptographiques d'un utilisateur
    pub async fn get_crypto_keys(&self, keys_id: Uuid) -> Result<CryptographicKeys> {
        let keys_row = sqlx::query(r#"
            SELECT id, user_id, public_key, encrypted_private_key, encryption_nonce,
                   encryption_salt, generated_at, last_used_at, key_version
            FROM cryptographic_keys WHERE id = ?
        "#)
        .bind(keys_id)
        .fetch_optional(&self.pool)
        .await?;

        let keys_row = keys_row.ok_or_else(|| anyhow!("Clés cryptographiques non trouvées"))?;

        Ok(CryptographicKeys {
            id: keys_row.get("id"),
            user_id: keys_row.get("user_id"),
            public_key: keys_row.get("public_key"),
            encrypted_private_key: keys_row.get("encrypted_private_key"),
            encryption_nonce: keys_row.get("encryption_nonce"),
            encryption_salt: keys_row.get("encryption_salt"),
            generated_at: keys_row.get("generated_at"),
            last_used_at: keys_row.get("last_used_at"),
            key_version: keys_row.get("key_version"),
        })
    }

    /// Récupère un dossier de validation d'identité
    async fn get_identity_validation(&self, validation_id: Uuid) -> Result<IdentityValidation> {
        let validation_row = sqlx::query(r#"
            SELECT * FROM identity_validations WHERE id = ?
        "#)
        .bind(validation_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = validation_row.ok_or_else(|| anyhow!("Dossier de validation non trouvé"))?;

        Ok(IdentityValidation {
            id: row.get("id"),
            user_id: row.get("user_id"),
            document_type: row.get("document_type"),
            document_number: row.get("document_number"),
            document_name: row.get("document_name"),
            document_firstname: row.get("document_firstname"),
            document_birth_date: row.get("document_birth_date"),
            document_file_path: row.get("document_file_path"),
            government_api_transaction_id: row.get("government_api_transaction_id"),
            government_api_response: row.get("government_api_response"),
            status: self.parse_identity_status(row.get("status"))?,
            submitted_at: row.get("submitted_at"),
            validated_at: row.get("validated_at"),
            rejection_reason: row.get("rejection_reason"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    // ========== Utilitaires privés ==========

    fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Erreur de hashage: {}", e))?;
        Ok(password_hash.to_string())
    }

    fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow!("Hash invalide: {}", e))?;
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    fn derive_key_from_password(&self, password: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
        use argon2::{Argon2, PasswordHasher};
        
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(salt).map_err(|e| anyhow!("Erreur de salt: {}", e))?;
        let password_hash = argon2.hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| anyhow!("Erreur de dérivation de clé: {}", e))?;
        
        let hash_bytes = password_hash.hash.unwrap();
        let key_bytes = hash_bytes.as_bytes();
        Ok(*Key::<Aes256Gcm>::from_slice(&key_bytes[..32]))
    }

    fn parse_user_role(&self, role_str: &str) -> Result<UserRole> {
        match role_str {
            "citizen" => Ok(UserRole::Citizen),
            "government" => Ok(UserRole::Government),
            "administrator" => Ok(UserRole::Administrator),
            _ => Err(anyhow!("Rôle utilisateur invalide: {}", role_str)),
        }
    }

    fn parse_identity_status(&self, status_str: &str) -> Result<IdentityStatus> {
        match status_str {
            "pending" => Ok(IdentityStatus::Pending),
            "submitted" => Ok(IdentityStatus::Submitted),
            "validating" => Ok(IdentityStatus::Validating),
            "validated" => Ok(IdentityStatus::Validated),
            "rejected" => Ok(IdentityStatus::Rejected),
            "expired" => Ok(IdentityStatus::Expired),
            _ => Err(anyhow!("Statut d'identité invalide: {}", status_str)),
        }
    }
}