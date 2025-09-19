-- Création de la table des utilisateurs
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'citizen', -- citizen, representative, administrator
    identity_status TEXT NOT NULL DEFAULT 'pending', -- pending, submitted, validating, validated, rejected, expired
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    identity_validation_id TEXT,
    crypto_keys_id TEXT,
    FOREIGN KEY (identity_validation_id) REFERENCES identity_validations(id),
    FOREIGN KEY (crypto_keys_id) REFERENCES cryptographic_keys(id)
);

-- Création de la table des validations d'identité
CREATE TABLE IF NOT EXISTS identity_validations (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    document_type TEXT NOT NULL, -- CNI, Passeport, etc.
    document_number TEXT NOT NULL,
    document_name TEXT NOT NULL,
    document_firstname TEXT NOT NULL,
    document_birth_date TEXT NOT NULL, -- Format YYYY-MM-DD
    document_file_path TEXT, -- Chemin vers le fichier chiffré
    government_api_transaction_id TEXT, -- ID de transaction avec l'API gouvernementale
    government_api_response TEXT, -- Réponse JSON de l'API
    status TEXT NOT NULL DEFAULT 'pending', -- pending, submitted, validating, validated, rejected, expired
    submitted_at TEXT,
    validated_at TEXT,
    rejection_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Création de la table des clés cryptographiques
CREATE TABLE IF NOT EXISTS cryptographic_keys (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    public_key BLOB NOT NULL, -- Clé publique Ed25519 (32 bytes)
    encrypted_private_key BLOB NOT NULL, -- Clé privée chiffrée avec AES-GCM
    encryption_nonce BLOB NOT NULL, -- Nonce pour AES-GCM (12 bytes)
    encryption_salt BLOB NOT NULL, -- Salt pour dérivation de clé (32 bytes)
    generated_at TEXT NOT NULL,
    last_used_at TEXT,
    key_version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Création de la table des sessions utilisateur
CREATE TABLE IF NOT EXISTS user_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL, -- Hash SHA-256 du token JWT
    ip_address TEXT,
    user_agent TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index pour améliorer les performances
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_identity_status ON users(identity_status);
CREATE INDEX IF NOT EXISTS idx_identity_validations_user_id ON identity_validations(user_id);
CREATE INDEX IF NOT EXISTS idx_identity_validations_status ON identity_validations(status);
CREATE INDEX IF NOT EXISTS idx_cryptographic_keys_user_id ON cryptographic_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_user_id ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_sessions_token_hash ON user_sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_user_sessions_expires_at ON user_sessions(expires_at);

-- Insertion des utilisateurs de démonstration (temporaire pour tests)
-- ATTENTION: Ces utilisateurs seront supprimés en production
INSERT INTO users (id, email, name, password_hash, role, identity_status, created_at, updated_at) VALUES
(
    '550e8400-e29b-41d4-a716-446655440001',
    'marie@example.com',
    'Marie Dupont',
    '$argon2id$v=19$m=65536,t=2,p=1$SomeBase64Salt$SomeBase64Hash', -- Placeholder - sera remplacé
    'citizen',
    'pending',
    datetime('now'),
    datetime('now')
),
(
    '550e8400-e29b-41d4-a716-446655440002', 
    'jean@example.com',
    'Jean Martin',
    '$argon2id$v=19$m=65536,t=2,p=1$SomeBase64Salt$SomeBase64Hash', -- Placeholder - sera remplacé
    'representative',
    'pending',
    datetime('now'),
    datetime('now')
),
(
    '550e8400-e29b-41d4-a716-446655440003',
    'admin@example.com', 
    'Administrateur Système',
    '$argon2id$v=19$m=65536,t=2,p=1$SomeBase64Salt$SomeBase64Hash', -- Placeholder - sera remplacé
    'administrator',
    'validated', -- Admin directement validé
    datetime('now'),
    datetime('now')
);