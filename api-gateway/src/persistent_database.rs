use anyhow::Result;
use sqlx::{SqlitePool, Row};
use sqlx::sqlite::SqliteRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::*;

/// Service de base de données pour la gestion des utilisateurs
#[derive(Clone, Debug)]
pub struct DatabaseService {
    pool: SqlitePool,
}

impl DatabaseService {
    fn row_to_user(row: SqliteRow) -> Result<User> {
        let id_str: String = row.try_get("id")?;
        let user_id = Uuid::parse_str(&id_str)?;

        let email: String = row.try_get("email")?;
        let name: String = row.try_get("name")?;
        let password_hash: String = row.try_get("password_hash")?;

        let role_str: String = row.try_get("role")?;
        let role = match role_str.as_str() {
            "citizen" => common::UserRole::Citizen,
            // Back-compat: accept both 'government' and legacy 'representative'
            "government" | "representative" => common::UserRole::Government,
            "administrator" => common::UserRole::Administrator,
            other => return Err(anyhow::anyhow!("Unknown role value: {}", other)),
        };

        let status_str: String = row.try_get("identity_status")?;
        let identity_status = match status_str.as_str() {
            "pending" => IdentityStatus::Pending,
            "submitted" => IdentityStatus::Submitted,
            "validating" => IdentityStatus::Validating,
            "validated" => IdentityStatus::Validated,
            "rejected" => IdentityStatus::Rejected,
            "expired" => IdentityStatus::Expired,
            other => return Err(anyhow::anyhow!("Unknown identity_status: {}", other)),
        };

        let created_at_str: String = row.try_get("created_at")?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at_str: String = row.try_get("updated_at")?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        let identity_validation_id: Option<String> = row.try_get("identity_validation_id")?;
        let identity_validation_id = match identity_validation_id {
            Some(s) if !s.is_empty() => Some(Uuid::parse_str(&s)?),
            _ => None,
        };

        let crypto_keys_id: Option<String> = row.try_get("crypto_keys_id")?;
        let crypto_keys_id = match crypto_keys_id {
            Some(s) if !s.is_empty() => Some(Uuid::parse_str(&s)?),
            _ => None,
        };

        Ok(User {
            id: user_id,
            email,
            name,
            password_hash,
            role,
            identity_status,
            created_at,
            updated_at,
            identity_validation_id,
            crypto_keys_id,
        })
    }
    /// Créer une nouvelle instance du service de base de données
    pub async fn new(database_url: &str) -> Result<Self> {
        // Afficher la version utilisée de DATABASE_URL
        println!("DATABASE_URL reçue: {}", database_url);
        
        // Normaliser l'URL de la base de données
        let db_url = if database_url.starts_with("sqlite:") {
            database_url.to_string()
        } else {
            format!("sqlite:{}", database_url)
        };
        
        println!("URL de base de données normalisée: {}", db_url);
        
        // Si ce n'est pas une base de données en mémoire, vérifier le chemin
        let is_memory_db = db_url == "sqlite::memory:";
        
        if !is_memory_db {
            if let Some(file_path) = db_url.strip_prefix("sqlite:") {
                let path = std::path::Path::new(file_path);
                
                // Si le chemin est relatif ou dans un sous-répertoire
                if let Some(parent_dir) = path.parent() {
                    if parent_dir.as_os_str().len() > 0 && !parent_dir.exists() {
                        println!("Création du répertoire: {:?}", parent_dir);
                        std::fs::create_dir_all(parent_dir)?;
                    }
                }
                
                // S'assurer que nous pouvons créer le fichier ou vérifier qu'il existe
                if path.exists() {
                    println!("Base de données existante trouvée: {:?}", path);
                    
                    // Tester les permissions
                    let metadata = std::fs::metadata(path);
                    if let Ok(meta) = metadata {
                        println!("Permissions: {:?}", meta.permissions());
                    }
                } else {
                    // Toucher le fichier pour s'assurer qu'il est créable
                    match std::fs::File::create(path) {
                        Ok(file) => {
                            drop(file); // Fermer le fichier immédiatement
                            println!("Fichier de base de données créé: {:?}", path);
                        },
                        Err(e) => {
                            println!("Erreur lors de la création du fichier de base de données: {:?}", e);
                            return Err(anyhow::anyhow!("Impossible de créer le fichier de base de données: {}", e));
                        }
                    }
                }
            }
        }
        
        // Créer le pool de connexion
        println!("Création du pool de connexion...");
        let pool = if is_memory_db {
            println!("Utilisation d'une base de données SQLite en mémoire PARTAGÉE");
            
            // URL pour une base de données en mémoire partagée entre les connexions
            // SOLUTION: Utiliser file:memdb?mode=memory&cache=shared au lieu de :memory:
            let shared_memory_url = "file:memdb1?mode=memory&cache=shared";
            
            // Options spécifiques pour SQLite en mémoire partagée
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(shared_memory_url)  // URL partagée au lieu de :memory:
                .create_if_missing(true)
                .foreign_keys(true);
                
            SqlitePool::connect_with(options).await?
        } else {
            // Options pour s'assurer que la base de données est créée
            let db_path = db_url.strip_prefix("sqlite:").unwrap_or("database.db");
            println!("Chemin de la base de données SQLite: {}", db_path);
            
            let options = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

            SqlitePool::connect_with(options).await?
        };
        
        println!("Pool de connexion créé avec succès");
        println!("Exécution des migrations...");
        
        // Toujours créer les tables manuellement pour s'assurer qu'elles existent
        println!("Création des tables pour la base de données...");
        
        // Création de la table users (en tenant compte des deux fichiers de migration)
        let user_table_result = sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('citizen', 'representative', 'administrator')),
                identity_status TEXT NOT NULL DEFAULT 'pending' CHECK (
                    identity_status IN ('pending', 'submitted', 'validating', 'validated', 'rejected', 'expired')
                ),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                identity_validation_id TEXT,
                crypto_keys_id TEXT
            );"
        ).execute(&pool).await;
        
        if let Err(e) = &user_table_result {
            println!("Erreur lors de la création de la table users: {:?}", e);
        } else {
            println!("Table users créée avec succès");
        }
        
        // Création de la table identity_validations
        let id_validation_result = sqlx::query(
            "CREATE TABLE IF NOT EXISTS identity_validations (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                document_type TEXT NOT NULL,
                document_number TEXT NOT NULL,
                document_name TEXT NOT NULL,
                document_firstname TEXT NOT NULL,
                document_birth_date TEXT NOT NULL,
                document_file_path TEXT,
                government_api_transaction_id TEXT,
                government_api_response TEXT,
                status TEXT NOT NULL DEFAULT 'pending' CHECK (
                    status IN ('pending', 'submitted', 'validating', 'validated', 'rejected', 'expired')
                ),
                submitted_at TEXT,
                validated_at TEXT,
                rejection_reason TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );"
        ).execute(&pool).await;
        
        if let Err(e) = &id_validation_result {
            println!("Erreur lors de la création de la table identity_validations: {:?}", e);
        } else {
            println!("Table identity_validations créée avec succès");
        }
        
        // Création de la table cryptographic_keys
        let crypto_keys_result = sqlx::query(
            "CREATE TABLE IF NOT EXISTS cryptographic_keys (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                public_key BLOB NOT NULL,
                encrypted_private_key BLOB NOT NULL,
                encryption_nonce BLOB NOT NULL,
                encryption_salt BLOB NOT NULL,
                generated_at TEXT NOT NULL,
                last_used_at TEXT,
                key_version INTEGER NOT NULL DEFAULT 1
            );"
        ).execute(&pool).await;
        
        if let Err(e) = &crypto_keys_result {
            println!("Erreur lors de la création de la table cryptographic_keys: {:?}", e);
        } else {
            println!("Table cryptographic_keys créée avec succès");
        }
        
        // Création de la table user_sessions
        let sessions_result = sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                ip_address TEXT,
                user_agent TEXT,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                revoked_at TEXT
            );"
        ).execute(&pool).await;
        
        if let Err(e) = &sessions_result {
            println!("Erreur lors de la création de la table user_sessions: {:?}", e);
        } else {
            println!("Table user_sessions créée avec succès");
        }
        
        // Ajout des index - ignorer les erreurs car ils peuvent déjà exister
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_identity_status ON users(identity_status);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_identity_validations_user_id ON identity_validations(user_id);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_identity_validations_status ON identity_validations(status);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_cryptographic_keys_user_id ON cryptographic_keys(user_id);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_sessions_token_hash ON user_sessions(token_hash);").execute(&pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_sessions_expires_at ON user_sessions(expires_at);").execute(&pool).await;
        
        println!("Initialisation de la base de données terminée");
        
        // S'il y a une erreur critique pour l'une des tables principales, la renvoyer
        user_table_result?;
        id_validation_result?;
        crypto_keys_result?;
        sessions_result?;
        
        // La vérification des migrations est maintenant faite directement
        println!("Initialisation de la base de données réussie!");
        
        Ok(Self { pool })
    }

    /// Expose the connection pool for testing
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Créer un nouvel utilisateur
    pub async fn create_user(&self, email: &str, name: &str, password_hash: &str) -> Result<User> {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Insert the user
        sqlx::query(
            r#"
            INSERT INTO users (id, email, name, password_hash, role, identity_status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 'citizen', 'pending', ?5, ?6)
            "#
        )
        .bind(user_id.to_string())
        .bind(email)
        .bind(name)
        .bind(password_hash)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        // Return the inserted user
        let row = sqlx::query(
            r#"
            SELECT id, email, name, password_hash, role, identity_status,
                   created_at, updated_at, identity_validation_id, crypto_keys_id
            FROM users WHERE id = ?1
            "#
        )
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        let user = Self::row_to_user(row)?;

        Ok(user)
    }

    /// Trouver un utilisateur par email
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row_opt = sqlx::query(
            r#"
            SELECT id, email, name, password_hash, role, identity_status,
                   created_at, updated_at, identity_validation_id, crypto_keys_id
            FROM users WHERE email = ?1
            "#
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        let user = match row_opt {
            Some(row) => Some(Self::row_to_user(row)?),
            None => None,
        };

        Ok(user)
    }

    /// Trouver un utilisateur par ID
    pub async fn find_user_by_id(&self, user_id: &Uuid) -> Result<Option<User>> {
        let row_opt = sqlx::query(
            r#"
            SELECT id, email, name, password_hash, role, identity_status,
                   created_at, updated_at, identity_validation_id, crypto_keys_id
            FROM users WHERE id = ?1
            "#
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let user = match row_opt {
            Some(row) => Some(Self::row_to_user(row)?),
            None => None,
        };

        Ok(user)
    }

    /// Mettre à jour le statut d'identité d'un utilisateur
    pub async fn update_user_identity_status(&self, user_id: &Uuid, status: IdentityStatus) -> Result<()> {
        let status_str = match status {
            IdentityStatus::Pending => "pending",
            IdentityStatus::Submitted => "submitted",
            IdentityStatus::Validating => "validating",
            IdentityStatus::Validated => "validated",
            IdentityStatus::Rejected => "rejected",
            IdentityStatus::Expired => "expired",
        };

        let now_str = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE users SET identity_status = ?1, updated_at = ?2 WHERE id = ?3"
        )
        .bind(status_str)
        .bind(now_str)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Créer une demande de validation d'identité
    pub async fn create_identity_validation(&self, user_id: &Uuid, request: &SubmitIdentityRequest) -> Result<IdentityValidation> {
        let validation_id = Uuid::new_v4();
        let now = Utc::now();

        let doc_date_str = request.document_birth_date.to_string();
        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO identity_validations (
                id, user_id, document_type, document_number, document_name, 
                document_firstname, document_birth_date, status, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'submitted', ?8, ?9)
            "#
        )
        .bind(validation_id.to_string())
        .bind(user_id.to_string())
        .bind(&request.document_type)
        .bind(&request.document_number)
        .bind(&request.document_name)
        .bind(&request.document_firstname)
        .bind(&doc_date_str)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&self.pool)
        .await?;

        // Mettre à jour la référence dans users
        sqlx::query(
            "UPDATE users SET identity_validation_id = ?1, identity_status = 'submitted', updated_at = ?2 WHERE id = ?3"
        )
    .bind(validation_id.to_string())
    .bind(&now_str)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        let validation = sqlx::query_as::<_, IdentityValidation>(
            r#"
            SELECT id, user_id, document_type, document_number, document_name,
                   document_firstname, document_birth_date, document_file_path,
                   government_api_transaction_id, government_api_response,
                   status, submitted_at, validated_at, rejection_reason,
                   created_at, updated_at
            FROM identity_validations WHERE id = ?1
            "#
        )
        .bind(validation_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(validation)
    }

    /// Récupérer une validation d'identité par user_id
    pub async fn get_identity_validation(&self, user_id: &Uuid) -> Result<Option<IdentityValidation>> {
        let validation = sqlx::query_as::<_, IdentityValidation>(
            r#"
            SELECT id, user_id, document_type, document_number, document_name,
                   document_firstname, document_birth_date, document_file_path,
                   government_api_transaction_id, government_api_response,
                   status, submitted_at, validated_at, rejection_reason,
                   created_at, updated_at
            FROM identity_validations WHERE user_id = ?1
            ORDER BY created_at DESC LIMIT 1
            "#
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(validation)
    }

    /// Mettre à jour le statut d'une validation d'identité
    pub async fn update_identity_validation_status(
        &self, 
        validation_id: &Uuid, 
        status: IdentityStatus,
        government_response: Option<&str>,
        rejection_reason: Option<&str>
    ) -> Result<()> {
        let status_str = match status {
            IdentityStatus::Pending => "pending",
            IdentityStatus::Submitted => "submitted", 
            IdentityStatus::Validating => "validating",
            IdentityStatus::Validated => "validated",
            IdentityStatus::Rejected => "rejected",
            IdentityStatus::Expired => "expired",
        };

        let now = Utc::now();
        let validated_at = if matches!(status, IdentityStatus::Validated) {
            Some(now.to_rfc3339())
        } else {
            None
        };

        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE identity_validations 
            SET status = ?1, government_api_response = ?2, rejection_reason = ?3, 
                validated_at = ?4, updated_at = ?5
            WHERE id = ?6
            "#
        )
        .bind(status_str)
        .bind(government_response)
        .bind(rejection_reason)
        .bind(validated_at)
        .bind(now_str)
        .bind(validation_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Créer des clés cryptographiques pour un utilisateur
    pub async fn create_crypto_keys(&self, user_id: &Uuid, public_key: &[u8], encrypted_private_key: &[u8], nonce: &[u8], salt: &[u8]) -> Result<CryptographicKeys> {
        let keys_id = Uuid::new_v4();
        let now = Utc::now();

        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO cryptographic_keys (
                id, user_id, public_key, encrypted_private_key, 
                encryption_nonce, encryption_salt, generated_at, key_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)
            "#
        )
        .bind(keys_id.to_string())
        .bind(user_id.to_string())
        .bind(public_key)
        .bind(encrypted_private_key)
        .bind(nonce)
        .bind(salt)
    .bind(&now_str)
        .execute(&self.pool)
        .await?;

        // Mettre à jour la référence dans users
        sqlx::query(
            "UPDATE users SET crypto_keys_id = ?1, updated_at = ?2 WHERE id = ?3"
        )
    .bind(keys_id.to_string())
    .bind(&now_str)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        let keys = sqlx::query_as::<_, CryptographicKeys>(
            r#"
            SELECT id, user_id, public_key, encrypted_private_key,
                   encryption_nonce, encryption_salt, generated_at,
                   last_used_at, key_version
            FROM cryptographic_keys WHERE id = ?1
            "#
        )
        .bind(keys_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Récupérer les clés cryptographiques d'un utilisateur
    pub async fn get_crypto_keys(&self, user_id: &Uuid) -> Result<Option<CryptographicKeys>> {
        let keys = sqlx::query_as::<_, CryptographicKeys>(
            r#"
            SELECT id, user_id, public_key, encrypted_private_key,
                   encryption_nonce, encryption_salt, generated_at,
                   last_used_at, key_version
            FROM cryptographic_keys WHERE user_id = ?1
            ORDER BY key_version DESC LIMIT 1
            "#
        )
        .bind(user_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(keys)
    }

    /// Créer une session utilisateur
    pub async fn create_user_session(&self, user_id: &Uuid, token_hash: &str, expires_at: DateTime<Utc>, ip_address: Option<&str>, user_agent: Option<&str>) -> Result<UserSession> {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let now_str = now.to_rfc3339();
        let exp_str = expires_at.to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO user_sessions (
                id, user_id, token_hash, ip_address, user_agent, 
                created_at, expires_at, is_active
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)
            "#
        )
        .bind(session_id.to_string())
        .bind(user_id.to_string())
        .bind(token_hash)
        .bind(ip_address)
        .bind(user_agent)
        .bind(now_str)
        .bind(exp_str)
        .execute(&self.pool)
        .await?;

        let session = sqlx::query_as::<_, UserSession>(
            r#"
            SELECT id, user_id, token_hash, ip_address, user_agent,
                   created_at, expires_at, is_active, revoked_at
            FROM user_sessions WHERE id = ?1
            "#
        )
        .bind(session_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Vérifier si une session est valide
    pub async fn is_session_valid(&self, token_hash: &str) -> Result<Option<UserSession>> {
        let now_str = Utc::now().to_rfc3339();
        let session = sqlx::query_as::<_, UserSession>(
            r#"
            SELECT id, user_id, token_hash, ip_address, user_agent,
                   created_at, expires_at, is_active, revoked_at
            FROM user_sessions 
            WHERE token_hash = ?1 AND is_active = 1 AND expires_at > ?2
            "#
        )
        .bind(token_hash)
        .bind(now_str)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Révoquer une session
    pub async fn revoke_session(&self, token_hash: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE user_sessions SET is_active = 0, revoked_at = ?1 WHERE token_hash = ?2"
        )
        .bind(now_str)
        .bind(token_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Révoquer toutes les sessions d'un utilisateur
    pub async fn revoke_all_user_sessions(&self, user_id: &Uuid) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE user_sessions SET is_active = 0, revoked_at = ?1 WHERE user_id = ?2"
        )
        .bind(now_str)
        .bind(user_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}