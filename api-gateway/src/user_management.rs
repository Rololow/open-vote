use anyhow::{Result, anyhow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use base64::Engine;

use crate::database::DatabaseService;
use crate::identity_validation::{IdentityValidationService, IdentityValidationConfig, ValidationResult};
use crate::models::{
    User, IdentityValidation, CryptographicKeys, UserProfile, LoginResponse, IdentityStatus,
    RegisterRequest, LoginRequest, SubmitIdentityRequest, IdentityValidationStatus, CryptoKeysInfo
};

/// Service intégré de gestion des utilisateurs avec validation d'identité
#[derive(Debug, Clone)]
pub struct UserManagementService {
    database: Arc<DatabaseService>,
    identity_validator: Arc<IdentityValidationService>,
}

impl UserManagementService {
    /// Initialise le service de gestion des utilisateurs
    pub async fn new(database_url: &str, identity_config: IdentityValidationConfig) -> Result<Self> {
        let database = Arc::new(DatabaseService::new(database_url).await?);
        let identity_validator = Arc::new(IdentityValidationService::new(identity_config));
        
        Ok(Self {
            database,
            identity_validator,
        })
    }

    /// Workflow complet d'inscription d'un nouvel utilisateur
    pub async fn register_user(&self, request: &RegisterRequest) -> Result<UserProfile> {
        // Validation des données d'entrée
        self.validate_registration_data(request)?;
        
        // Créer l'utilisateur en base
        let user = self.database.create_user(request).await?;
        
        // Convertir en profil utilisateur
        Ok(user.to_profile(false))
    }

    /// Workflow complet d'authentification utilisateur
    pub async fn authenticate_user(&self, request: &LoginRequest) -> Result<LoginResponse> {
        // Authentifier via la base de données
        let user = self.database.authenticate_user(request).await?;
        
        // Vérifier si l'utilisateur a des clés cryptographiques
        let has_crypto_keys = user.crypto_keys_id.is_some();
        
        // Générer un token JWT (temporaire - à améliorer avec session management)
        let token = self.generate_jwt_token(&user)?;
        let expires_at = Utc::now() + chrono::Duration::hours(24);
        
        Ok(LoginResponse {
            token,
            user: user.to_profile(has_crypto_keys),
            expires_at,
        })
    }

    /// Soumet des documents pour validation d'identité
    pub async fn submit_identity_documents(&self, user_id: Uuid, request: &SubmitIdentityRequest) -> Result<IdentityValidationStatus> {
        // Validation des données d'entrée
        self.validate_identity_submission(request)?;
        
        // Créer le dossier de validation
        let validation = self.database.submit_identity_validation(user_id, request).await?;
        
        // Lancer la validation asynchrone en arrière-plan
        tokio::spawn({
            let validation_service = Arc::clone(&self.identity_validator);
            let database_service = Arc::clone(&self.database);
            let validation_clone = validation.clone();
            
            async move {
                if let Err(e) = Self::process_identity_validation(
                    validation_service,
                    database_service,
                    validation_clone
                ).await {
                    tracing::error!("Erreur lors de la validation d'identité: {}", e);
                }
            }
        });
        
        Ok(validation.to_status())
    }

    /// Processus de validation d'identité en arrière-plan
    async fn process_identity_validation(
        validator: Arc<IdentityValidationService>,
        database: Arc<DatabaseService>,
        mut validation: IdentityValidation,
    ) -> Result<()> {
        // Mettre à jour le statut en "validation en cours"
        validation.status = IdentityStatus::Validating;
        // TODO: Mettre à jour en base
        
        // Appeler l'API de validation
        let result = validator.validate_identity(&validation).await?;
        
        // Mettre à jour le dossier avec le résultat
        validation.status = result.to_identity_status();
        validation.government_api_transaction_id = result.transaction_id.clone();
        validation.government_api_response = Some(serde_json::to_string(&result).unwrap_or_default());
        
        match result.is_valid {
            true => {
                validation.validated_at = Some(Utc::now());
                // TODO: Mettre à jour l'utilisateur en "validated"
                tracing::info!("Validation d'identité réussie pour l'utilisateur {}", validation.user_id);
            }
            false => {
                validation.rejection_reason = result.error_message;
                tracing::warn!("Validation d'identité échouée pour l'utilisateur {}: {}", 
                    validation.user_id, 
                    validation.rejection_reason.as_deref().unwrap_or("Raison inconnue")
                );
            }
        }
        
        // TODO: Sauvegarder les modifications en base
        
        Ok(())
    }

    /// Génère les clés cryptographiques pour un utilisateur validé
    pub async fn generate_crypto_keys(&self, user_id: Uuid, password: &str) -> Result<CryptoKeysInfo> {
        let keys = self.database.generate_crypto_keys(user_id, password).await?;
        
        Ok(CryptoKeysInfo {
            has_keys: true,
            public_key_hex: Some(hex::encode(&keys.public_key)),
            generated_at: Some(keys.generated_at),
            last_used_at: keys.last_used_at,
            key_version: Some(keys.key_version),
        })
    }

    /// Récupère le profil complet d'un utilisateur
    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<UserProfile> {
        let user = self.database.get_user_by_id(user_id).await?;
        let has_crypto_keys = user.crypto_keys_id.is_some();
        
        Ok(user.to_profile(has_crypto_keys))
    }

    /// Récupère le statut de validation d'identité d'un utilisateur
    pub async fn get_identity_validation_status(&self, user_id: Uuid) -> Result<Option<IdentityValidationStatus>> {
        let user = self.database.get_user_by_id(user_id).await?;
        
        if let Some(validation_id) = user.identity_validation_id {
            // TODO: Récupérer le dossier de validation
            // Pour l'instant, retourner un statut basé sur l'utilisateur
            Ok(Some(IdentityValidationStatus {
                status: user.identity_status.clone(),
                submitted_at: Some(user.created_at), // Placeholder
                validated_at: None, // TODO: Récupérer depuis le dossier
                rejection_reason: None,
                can_resubmit: matches!(user.identity_status, IdentityStatus::Rejected | IdentityStatus::Expired),
            }))
        } else {
            Ok(None)
        }
    }

    /// Récupère les informations des clés cryptographiques d'un utilisateur
    pub async fn get_crypto_keys_info(&self, user_id: Uuid) -> Result<CryptoKeysInfo> {
        let user = self.database.get_user_by_id(user_id).await?;
        
        if let Some(keys_id) = user.crypto_keys_id {
            let keys = self.database.get_crypto_keys(keys_id).await?;
            Ok(CryptoKeysInfo {
                has_keys: true,
                public_key_hex: Some(hex::encode(&keys.public_key)),
                generated_at: Some(keys.generated_at),
                last_used_at: keys.last_used_at,
                key_version: Some(keys.key_version),
            })
        } else {
            Ok(CryptoKeysInfo {
                has_keys: false,
                public_key_hex: None,
                generated_at: None,
                last_used_at: None,
                key_version: None,
            })
        }
    }

    /// Vérifie si un utilisateur peut voter
    pub async fn can_user_vote(&self, user_id: Uuid) -> Result<bool> {
        let profile = self.get_user_profile(user_id).await?;
        Ok(profile.can_vote)
    }

    // ========== Fonctions utilitaires privées ==========

    /// Valide les données d'inscription
    fn validate_registration_data(&self, request: &RegisterRequest) -> Result<()> {
        // Validation email
        if !self.is_valid_email(&request.email) {
            return Err(anyhow!("Format d'email invalide"));
        }
        
        // Validation nom
        if request.name.trim().len() < 2 {
            return Err(anyhow!("Le nom doit contenir au moins 2 caractères"));
        }
        
        // Validation mot de passe
        if request.password.len() < 8 {
            return Err(anyhow!("Le mot de passe doit contenir au moins 8 caractères"));
        }
        
        if !self.is_strong_password(&request.password) {
            return Err(anyhow!("Le mot de passe doit contenir au moins une majuscule, une minuscule et un chiffre"));
        }
        
        Ok(())
    }

    /// Valide la soumission de documents d'identité
    fn validate_identity_submission(&self, request: &SubmitIdentityRequest) -> Result<()> {
        // Validation type de document
        let valid_types = vec!["CNI", "PASSEPORT", "FRANCE_CONNECT"];
        if !valid_types.contains(&request.document_type.to_uppercase().as_str()) {
            return Err(anyhow!("Type de document non supporté: {}", request.document_type));
        }
        
        // Validation numéro de document
        if request.document_number.trim().len() < 6 {
            return Err(anyhow!("Numéro de document invalide"));
        }
        
        // Validation nom et prénom
        if request.document_name.trim().len() < 2 || request.document_firstname.trim().len() < 2 {
            return Err(anyhow!("Nom et prénom requis"));
        }
        
        // Validation date de naissance
        let today = chrono::Utc::now().date_naive();
        if request.document_birth_date > today {
            return Err(anyhow!("Date de naissance invalide"));
        }
        
        let age = today.years_since(request.document_birth_date).unwrap_or(0);
        if age < 18 || age > 120 {
            return Err(anyhow!("Âge invalide pour voter (doit être entre 18 et 120 ans)"));
        }
        
        Ok(())
    }

    /// Valide le format d'un email
    fn is_valid_email(&self, email: &str) -> bool {
        email.contains('@') && email.contains('.') && email.len() >= 5
    }

    /// Vérifie la force d'un mot de passe
    fn is_strong_password(&self, password: &str) -> bool {
        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        
        has_lower && has_upper && has_digit
    }

    /// Génère un token JWT temporaire (à améliorer)
    fn generate_jwt_token(&self, user: &User) -> Result<String> {
        // Pour l'instant, génération simple
        // TODO: Implémenter avec une vraie bibliothèque JWT et signature
        let payload = format!("{{\"user_id\":\"{}\",\"email\":\"{}\",\"exp\":{}}}", 
            user.id, 
            user.email,
            (Utc::now() + chrono::Duration::hours(24)).timestamp()
        );
        
        Ok(base64::engine::general_purpose::STANDARD.encode(payload))
    }

    /// Connecte un utilisateur (alias pour authenticate_user)
    pub async fn login_user(&self, request: &LoginRequest, ip_address: String, user_agent: String) -> Result<LoginResponse> {
        // Pour l'instant, ignore les métadonnées supplémentaires 
        // TODO: Ajouter logging de l'IP et user agent pour audit
        self.authenticate_user(request).await
    }

    /// Déconnecte un utilisateur en révoquant son token
    pub async fn logout_user(&self, token: &str) -> Result<()> {
        // TODO: Implémenter la révocation de token en base de données
        // Pour l'instant, on considère que c'est toujours succès
        tracing::info!("Utilisateur déconnecté avec token: {}", &token[..10]);
        Ok(())
    }

    /// Signe un message avec les clés cryptographiques de l'utilisateur
    pub async fn sign_message(&self, user_id: &Uuid, password: &str, message: &[u8]) -> Result<Vec<u8>> {
        // Récupérer les clés cryptographiques de l'utilisateur
        let crypto_keys = self.database.get_crypto_keys(*user_id).await?;
        
        // Utiliser le service crypto pour signer
        let crypto_service = crate::crypto_service::CryptographicService::new(self.database.clone().into());
        let signature = crypto_service.sign_with_user_keys(user_id, &crypto_keys, password, message).await?;
        
        Ok(signature)
    }

    /// Vérifie une signature avec la clé publique d'un utilisateur
    pub async fn verify_signature(&self, user_id: &Uuid, message: &[u8], signature: &[u8]) -> Result<bool> {
        // Récupérer les clés cryptographiques de l'utilisateur
        let crypto_keys = self.database.get_crypto_keys(*user_id).await?;
        
        // Utiliser le service crypto pour vérifier
        let crypto_service = crate::crypto_service::CryptographicService::new(self.database.clone().into());
        let is_valid = crypto_service.verify_signature(&crypto_keys.public_key, message, signature)?;
        
        Ok(is_valid)
    }

    /// Soumet un document d'identité (alias pour submit_identity_documents)
    pub async fn submit_identity_document(&self, user_id: &Uuid, request: &SubmitIdentityRequest) -> Result<IdentityValidationStatus> {
        self.submit_identity_documents(*user_id, request).await
    }

    /// Génère des clés cryptographiques pour un utilisateur (alias avec nom différent)
    pub async fn generate_user_crypto_keys(&self, user_id: &Uuid, password: &str) -> Result<CryptoKeysInfo> {
        self.generate_crypto_keys(*user_id, password).await
    }

    /// Obtient les informations des clés cryptographiques (alias avec nom différent)
    pub async fn get_user_crypto_keys_info(&self, user_id: &Uuid) -> Result<CryptoKeysInfo> {
        self.get_crypto_keys_info(*user_id).await
    }
}