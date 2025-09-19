-- Migration: Création des tables pour le système d'utilisateurs persistant
-- Version: 20250919000002
-- Description: Tables users, identity_validations, cryptographic_keys, user_sessions

-- Table des utilisateurs
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY, -- UUID as TEXT
    email TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    password_hash TEXT NOT NULL, -- Hash Argon2
    role TEXT NOT NULL CHECK (role IN ('citizen', 'representative', 'administrator')),
    identity_status TEXT NOT NULL DEFAULT 'pending' CHECK (
        identity_status IN ('pending', 'submitted', 'validating', 'validated', 'rejected', 'expired')
    ),
    created_at TEXT NOT NULL, -- ISO 8601 DateTime
    updated_at TEXT NOT NULL,
    identity_validation_id TEXT, -- UUID référence vers identity_validations
    crypto_keys_id TEXT, -- UUID référence vers cryptographic_keys
    FOREIGN KEY (identity_validation_id) REFERENCES identity_validations(id),
    FOREIGN KEY (crypto_keys_id) REFERENCES cryptographic_keys(id)
);

-- Table des validations d'identité
CREATE TABLE IF NOT EXISTS identity_validations (
    id TEXT PRIMARY KEY, -- UUID as TEXT
    user_id TEXT NOT NULL,
    document_type TEXT NOT NULL, -- 'CNI', 'Passeport', 'Permis'
    document_number TEXT NOT NULL,
    document_name TEXT NOT NULL,
    document_firstname TEXT NOT NULL,
    document_birth_date TEXT NOT NULL, -- Date format YYYY-MM-DD
    document_file_path TEXT, -- Chemin vers le fichier chiffré
    government_api_transaction_id TEXT, -- ID transaction API gouvernementale
    government_api_response TEXT, -- Réponse JSON de l'API
    status TEXT NOT NULL DEFAULT 'pending' CHECK (
        status IN ('pending', 'submitted', 'validating', 'validated', 'rejected', 'expired')
    ),
    submitted_at TEXT, -- DateTime ISO 8601
    validated_at TEXT, -- DateTime ISO 8601
    rejection_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Table des clés cryptographiques
CREATE TABLE IF NOT EXISTS cryptographic_keys (
    id TEXT PRIMARY KEY, -- UUID as TEXT
    user_id TEXT NOT NULL,
    public_key BLOB NOT NULL, -- Clé publique Ed25519 (32 bytes)
    encrypted_private_key BLOB NOT NULL, -- Clé privée chiffrée AES-GCM
    encryption_nonce BLOB NOT NULL, -- Nonce AES-GCM (12 bytes)
    encryption_salt BLOB NOT NULL, -- Salt pour dérivation clé (32 bytes)
    generated_at TEXT NOT NULL, -- DateTime ISO 8601
    last_used_at TEXT, -- DateTime ISO 8601
    key_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Table des sessions utilisateur
CREATE TABLE IF NOT EXISTS user_sessions (
    id TEXT PRIMARY KEY, -- UUID as TEXT
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL, -- Hash SHA-256 du JWT
    ip_address TEXT,
    user_agent TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index pour les performances
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_identity_status ON users(identity_status);
CREATE INDEX IF NOT EXISTS idx_identity_validations_user_id ON identity_validations(user_id);
CREATE INDEX IF NOT EXISTS idx_identity_validations_status ON identity_validations(status);
CREATE INDEX IF NOT EXISTS idx_identity_validations_document_number ON identity_validations(document_number);
CREATE INDEX IF NOT EXISTS idx_cryptographic_keys_user_id ON cryptographic_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_token_hash ON user_sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_user_sessions_expires_at ON user_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_user_sessions_active ON user_sessions(is_active) WHERE is_active = 1;

-- Contraintes de sécurité
-- SQLite ne supporte pas IF NOT EXISTS pour les triggers avant 3.35, mais on peut tenter la création
CREATE TRIGGER IF NOT EXISTS update_users_updated_at 
    AFTER UPDATE ON users
BEGIN
    UPDATE users SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_identity_validations_updated_at 
    AFTER UPDATE ON identity_validations
BEGIN
    UPDATE identity_validations SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger pour nettoyer les sessions expirées
CREATE TRIGGER IF NOT EXISTS cleanup_expired_sessions
    AFTER INSERT ON user_sessions
BEGIN
    DELETE FROM user_sessions 
    WHERE expires_at < datetime('now') AND is_active = 0;
END;