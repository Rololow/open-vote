use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use common::UserRole;

/// Statut de validation d'identité d'un utilisateur
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "identity_status", rename_all = "lowercase")]
pub enum IdentityStatus {
    /// En attente de soumission des documents
    Pending,
    /// Documents soumis, en cours de vérification
    Submitted,
    /// Validation en cours par l'API gouvernementale
    Validating,
    /// Identité validée avec succès
    Validated,
    /// Validation échouée - documents incorrects
    Rejected,
    /// Validation expirée - revalidation nécessaire
    Expired,
}

/// Utilisateur persistant en base de données
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    /// Hash Argon2 du mot de passe
    pub password_hash: String,
    pub role: UserRole,
    pub identity_status: IdentityStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Référence vers le dossier de validation d'identité
    pub identity_validation_id: Option<Uuid>,
    /// Référence vers les clés cryptographiques
    pub crypto_keys_id: Option<Uuid>,
}

/// Dossier de validation d'identité
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IdentityValidation {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Type de document (CNI, Passeport, etc.)
    pub document_type: String,
    /// Numéro du document d'identité
    pub document_number: String,
    /// Nom sur le document
    pub document_name: String,
    /// Prénom sur le document
    pub document_firstname: String,
    /// Date de naissance sur le document
    pub document_birth_date: chrono::NaiveDate,
    /// Chemin vers le scan/photo du document (chiffré)
    pub document_file_path: Option<String>,
    /// ID de transaction avec l'API gouvernementale
    pub government_api_transaction_id: Option<String>,
    /// Réponse de l'API gouvernementale
    pub government_api_response: Option<String>,
    pub status: IdentityStatus,
    pub submitted_at: Option<DateTime<Utc>>,
    pub validated_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Clés cryptographiques utilisateur (stockage sécurisé)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CryptographicKeys {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Clé publique Ed25519 (non chiffrée)
    pub public_key: Vec<u8>,
    /// Clé privée Ed25519 chiffrée avec AES-GCM
    pub encrypted_private_key: Vec<u8>,
    /// Nonce pour le chiffrement AES-GCM
    pub encryption_nonce: Vec<u8>,
    /// Salt pour dériver la clé de chiffrement du mot de passe
    pub encryption_salt: Vec<u8>,
    /// Timestamp de génération des clés
    pub generated_at: DateTime<Utc>,
    /// Dernière utilisation des clés
    pub last_used_at: Option<DateTime<Utc>>,
    /// Version des clés (pour rotation future)
    pub key_version: i32,
}

/// Session utilisateur pour gestion des tokens
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Hash du token JWT
    pub token_hash: String,
    /// Adresse IP de connexion
    pub ip_address: Option<String>,
    /// User-Agent de connexion
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Session active ou révoquée
    pub is_active: bool,
    /// Timestamp de révocation
    pub revoked_at: Option<DateTime<Utc>>,
}

// ========== Structures de requête API ==========

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitIdentityRequest {
    pub document_type: String,
    pub document_number: String,
    pub document_name: String,
    pub document_firstname: String,
    pub document_birth_date: chrono::NaiveDate,
    /// Base64 du scan du document
    pub document_file_base64: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub identity_status: IdentityStatus,
    pub created_at: DateTime<Utc>,
    pub has_crypto_keys: bool,
    pub can_vote: bool,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserProfile,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct IdentityValidationStatus {
    pub status: IdentityStatus,
    pub submitted_at: Option<DateTime<Utc>>,
    pub validated_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub can_resubmit: bool,
}

#[derive(Debug, Serialize)]
pub struct CryptoKeysInfo {
    pub has_keys: bool,
    pub public_key_hex: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub key_version: Option<i32>,
}

impl User {
    /// Vérifie si l'utilisateur peut voter
    pub fn can_vote(&self) -> bool {
        matches!(self.identity_status, IdentityStatus::Validated) && 
        self.crypto_keys_id.is_some()
    }

    /// Convertit vers UserProfile pour l'API
    pub fn to_profile(&self, has_crypto_keys: bool) -> UserProfile {
        UserProfile {
            id: self.id,
            email: self.email.clone(),
            name: self.name.clone(),
            role: self.role.clone(),
            identity_status: self.identity_status.clone(),
            created_at: self.created_at,
            has_crypto_keys,
            can_vote: self.can_vote() && has_crypto_keys,
        }
    }
}

impl IdentityValidation {
    /// Vérifie si on peut resoumettre des documents
    pub fn can_resubmit(&self) -> bool {
        matches!(self.status, IdentityStatus::Rejected | IdentityStatus::Expired)
    }

    /// Convertit vers le statut API
    pub fn to_status(&self) -> IdentityValidationStatus {
        IdentityValidationStatus {
            status: self.status.clone(),
            submitted_at: self.submitted_at,
            validated_at: self.validated_at,
            rejection_reason: self.rejection_reason.clone(),
            can_resubmit: self.can_resubmit(),
        }
    }
}