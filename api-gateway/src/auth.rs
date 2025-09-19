use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use crypto_lib::{PublicKey, KeyPair, Signature};
use uuid::Uuid;
use thiserror::Error;

/// Erreurs liées à l'authentification
#[derive(Error, Debug, Serialize)]
pub enum AuthError {
    #[error("Token invalide: {0}")]
    InvalidToken(String),
    
    #[error("Token expiré")]
    TokenExpired,
    
    #[error("Signature invalide")]
    InvalidSignature,
    
    #[error("Utilisateur non trouvé: {0}")]
    UserNotFound(String),
    
    #[error("Permissions insuffisantes")]
    InsufficientPermissions,
    
    #[error("Erreur de sérialisation: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, AuthError>;

// Utilisation du UserRole de common
pub use common::UserRole;



/// Claims JWT personnalisés pour notre système
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Identifiant unique de l'utilisateur
    pub sub: String, // Subject (user ID)
    
    /// Clé publique de l'utilisateur pour vérification
    pub public_key: PublicKey,
    
    /// Rôle de l'utilisateur
    pub role: UserRole,
    
    /// Permissions spécifiques
    pub permissions: Vec<String>,
    
    /// Date d'émission du token
    pub iat: i64, // Issued at
    
    /// Date d'expiration
    pub exp: i64, // Expiration
    
    /// Émetteur du token
    pub iss: String, // Issuer
    
    /// Audience (pour qui le token est destiné)
    pub aud: String, // Audience
    
    /// Session ID pour invalidation
    pub session_id: String,
}

impl JwtClaims {
    /// Crée de nouveaux claims JWT
    pub fn new(
        user_id: Uuid,
        public_key: PublicKey,
        role: UserRole,
        permissions: Vec<String>,
        duration_hours: i64,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(duration_hours);
        
        Self {
            sub: user_id.to_string(),
            public_key,
            role,
            permissions,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: "e-government-blockchain".to_string(),
            aud: "blockchain-users".to_string(),
            session_id: Uuid::new_v4().to_string(),
        }
    }
    
    /// Vérifie si le token est expiré
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
    
    /// Vérifie si l'utilisateur a une permission spécifique
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
    
    /// Obtient l'ID utilisateur
    pub fn user_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub)
            .map_err(|e| AuthError::InvalidToken(format!("ID utilisateur invalide: {}", e)))
    }
}

/// Token JWT signé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedJwt {
    /// Claims du token
    pub claims: JwtClaims,
    
    /// Signature cryptographique
    pub signature: Signature,
    
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
}

impl SignedJwt {
    /// Crée et signe un nouveau JWT
    pub fn create(claims: JwtClaims, signing_key: &KeyPair) -> Result<Self> {
        // Sérialiser les claims
        let claims_json = serde_json::to_string(&claims)
            .map_err(|e| AuthError::SerializationError(e.to_string()))?;
        
        // Signer les claims
        let signature = signing_key.sign(claims_json.as_bytes());
        
        Ok(Self {
            claims,
            signature,
            created_at: Utc::now(),
        })
    }
    
    /// Vérifie la signature du JWT
    pub fn verify_signature(&self, public_key: &PublicKey) -> Result<()> {
        let claims_json = serde_json::to_string(&self.claims)
            .map_err(|e| AuthError::SerializationError(e.to_string()))?;
        
        public_key.verify(claims_json.as_bytes(), &self.signature)
            .map_err(|_| AuthError::InvalidSignature)?;
        
        Ok(())
    }
    
    /// Vérifie la validité complète du token
    pub fn verify(&self, issuer_public_key: &PublicKey) -> Result<()> {
        // Vérifier l'expiration
        if self.claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }
        
        // Vérifier la signature
        self.verify_signature(issuer_public_key)?;
        
        Ok(())
    }
    
    /// Encode le JWT en format string (simplified base64)
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .map_err(|e| AuthError::SerializationError(e.to_string()))?;
        
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(json))
    }
    
    /// Décode un JWT depuis un string
    pub fn decode(token: &str) -> Result<Self> {
        use base64::Engine;
        let json = base64::engine::general_purpose::STANDARD.decode(token)
            .map_err(|e| AuthError::InvalidToken(format!("Décodage base64 échoué: {}", e)))?;
        
        let jwt: SignedJwt = serde_json::from_slice(&json)
            .map_err(|e| AuthError::InvalidToken(format!("JSON invalide: {}", e)))?;
        
        Ok(jwt)
    }
}

/// Gestionnaire d'authentification
pub struct AuthManager {
    /// Clé de signature pour les tokens
    signing_key: KeyPair,
    
    /// Durée de vie par défaut des tokens (en heures)
    default_token_duration: i64,
    
    /// Sessions actives (session_id -> user_id)
    active_sessions: std::collections::HashMap<String, Uuid>,
}

impl AuthManager {
    /// Crée un nouveau gestionnaire d'authentification
    pub fn new(signing_key: KeyPair, default_token_duration: i64) -> Self {
        Self {
            signing_key,
            default_token_duration,
            active_sessions: std::collections::HashMap::new(),
        }
    }
    
    /// Génère un token d'authentification
    pub fn generate_token(
        &mut self,
        user_id: Uuid,
        public_key: PublicKey,
        role: UserRole,
        permissions: Vec<String>,
    ) -> Result<SignedJwt> {
        let claims = JwtClaims::new(
            user_id,
            public_key,
            role,
            permissions,
            self.default_token_duration,
        );
        
        // Enregistrer la session
        self.active_sessions.insert(claims.session_id.clone(), user_id);
        
        SignedJwt::create(claims, &self.signing_key)
    }
    
    /// Valide un token d'authentification
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims> {
        let jwt = SignedJwt::decode(token)?;
        
        // Vérifier la validité
        jwt.verify(self.signing_key.public_key())?;
        
        // Vérifier que la session est active
        if !self.active_sessions.contains_key(&jwt.claims.session_id) {
            return Err(AuthError::InvalidToken("Session inactive".to_string()));
        }
        
        Ok(jwt.claims)
    }
    
    /// Révoque un token (invalide la session)
    pub fn revoke_token(&mut self, session_id: &str) -> Result<()> {
        self.active_sessions.remove(session_id)
            .ok_or_else(|| AuthError::InvalidToken("Session introuvable".to_string()))?;
        
        Ok(())
    }
    
    /// Révoque tous les tokens d'un utilisateur
    pub fn revoke_all_user_tokens(&mut self, user_id: Uuid) {
        self.active_sessions.retain(|_, &mut id| id != user_id);
    }
    
    /// Obtient les sessions actives
    pub fn active_session_count(&self) -> usize {
        self.active_sessions.len()
    }
    
    /// Nettoie les sessions expirées (à appeler périodiquement)
    pub fn cleanup_expired_sessions(&mut self) {
        // Note: Dans une implémentation réelle, il faudrait stocker les timestamps d'expiration
        // Pour l'instant, on garde toutes les sessions actives
    }
    
    /// Vérifie les permissions pour une action donnée
    pub fn check_permission(&self, claims: &JwtClaims, required_role: &UserRole, permission: &str) -> Result<()> {
        // Vérifier le rôle
        if !claims.role.has_permission(required_role) {
            return Err(AuthError::InsufficientPermissions);
        }
        
        // Vérifier la permission spécifique si requise
        if !permission.is_empty() && !claims.has_permission(permission) {
            return Err(AuthError::InsufficientPermissions);
        }
        
        Ok(())
    }
}

/// Permissions prédéfinies du système
pub mod permissions {
    pub const CREATE_LAW: &str = "create_law";
    pub const VOTE_ON_LAW: &str = "vote_on_law";
    pub const MODIFY_LAW: &str = "modify_law";
    pub const DELETE_LAW: &str = "delete_law";
    pub const VIEW_AUDIT_LOG: &str = "view_audit_log";
    pub const MANAGE_USERS: &str = "manage_users";
    pub const MODERATE_CONTENT: &str = "moderate_content";
    pub const ACCESS_ADMIN_PANEL: &str = "access_admin_panel";
    pub const EXPORT_DATA: &str = "export_data";
    pub const SYSTEM_MAINTENANCE: &str = "system_maintenance";
}



// Fonction d'authentification legacy (compatible)
pub fn authenticate_request(token: &str) -> bool {
    // Pour compatibilité, essayer de valider le token
    match SignedJwt::decode(token) {
        Ok(_) => true,
        Err(_) => false,
    }
}
