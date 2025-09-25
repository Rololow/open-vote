use anyhow::{Result, Context};
use sqlx::{SqlitePool, Row};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::path::Path;
use std::str::FromStr;
use tracing::{info, error};
use serde_json;
use std::ffi::OsStr;

use common::{Blockchain, Block, Transaction, Account, Law, Vote};

/// Gestionnaire de stockage pour la blockchain
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Crée une nouvelle instance de stockage
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("🗄️ Initialisation du stockage SQLite");
        info!("   📍 URL de la base de données: {}", database_url);

        // Ensure parent directory exists for file-based SQLite URLs
    if let Some(path_str) = database_url.strip_prefix("sqlite:") {
            info!("   📁 Chemin extrait: {}", path_str);
            // Accept both sqlite:./file and sqlite://./file forms
            let trimmed = path_str.trim_start_matches('/');
            // If it looks like a file path (starts with ./ or .\\ or lacks '?mode=memory')
            if !trimmed.starts_with(':') && !trimmed.starts_with("memory") {
                // Extract directory component and create it
                let file_path = if path_str.starts_with("//") {
                    // sqlite://./path -> strip the leading //
                    &path_str[2..]
                } else {
                    path_str
                };
                // Remove any query parameters
                let file_path = file_path.split('?').next().unwrap_or(file_path);
                // Normalize leading slashes introduced by //
                let file_path = file_path.trim_start_matches('/');
                let p = Path::new(file_path);
                if let Some(dir) = p.parent() {
                    if !dir.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(dir).await
                            .with_context(|| format!("Erreur création répertoire de données: {}", dir.display()))?;
                    }
                }

                // Pre-create the database file to avoid SQLITE_CANTOPEN (code 14)
                if !p.exists() {
                    use tokio::io::AsyncWriteExt;
                    if let Some(dir) = p.parent() {
                        tokio::fs::create_dir_all(dir).await.ok();
                    }
                    let mut f = tokio::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(p)
                        .await
                        .with_context(|| format!("Impossible de créer le fichier de base de données: {}", p.display()))?;
                    // Ensure the file exists; no content needed
                    f.flush().await.ok();
                }
            }
        }

        // Build connect options with create_if_missing to avoid open errors
        let options = SqliteConnectOptions::from_str(database_url)
            .context("Options de connexion SQLite invalides")?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Delete)
            .foreign_keys(true);

        info!("🔗 Connexion à la base de données SQLite...");
        let pool = SqlitePool::connect_with(options).await
            .context("Erreur connexion base de données")?;
        info!("✅ Connexion établie avec succès");

        let storage = Self { pool };
        
        // Initialiser les tables
        info!("🏗️ Initialisation des tables...");
        storage.init_tables().await?;
        info!("✅ Tables initialisées avec succès");
        
        Ok(storage)
    }

    /// Initialise les tables de la base de données
    async fn init_tables(&self) -> Result<()> {
        info!("Initialisation des tables via migrations SQL");
        // Exécuter les migrations (migrations/*.sql), y compris 0000_init.sql et 0001_identity_commitments.sql
        self.run_migrations().await?;
        info!("Tables initialisées avec succès (migrations appliquées)");
        Ok(())
    }

    /// Exécute les migrations SQL présentes dans le dossier migrations/ (ou MIGRATIONS_DIR)
    async fn run_migrations(&self) -> Result<()> {
        use tokio::fs;
        let dir = std::env::var("MIGRATIONS_DIR").unwrap_or_else(|_| "./migrations".to_string());
        let path = Path::new(&dir);
        if !path.exists() {
            info!("Aucun dossier de migrations trouvé: {} (skip)", dir);
            return Ok(());
        }

        // Table de suivi des migrations
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Récupérer liste des migrations déjà appliquées
        let applied_rows = sqlx::query("SELECT filename FROM schema_migrations")
            .fetch_all(&self.pool)
            .await?;
        let mut applied = std::collections::HashSet::new();
        for row in applied_rows { let f: String = row.get("filename"); applied.insert(f); }

        // Lister les fichiers .sql et trier
        let mut entries = fs::read_dir(path).await
            .with_context(|| format!("Lecture du dossier migrations: {}", dir))?;
        let mut files: Vec<String> = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let file_type = entry.file_type().await?;
            if file_type.is_file() {
                let name = entry.file_name();
                if Path::new(&name).extension() == Some(OsStr::new("sql")) {
                    files.push(name.to_string_lossy().to_string());
                }
            }
        }
        files.sort();

        // Appliquer chaque migration non encore appliquée
        for fname in files {
            if applied.contains(&fname) { continue; }
            let full = path.join(&fname);
            let sql = fs::read_to_string(&full).await
                .with_context(|| format!("Lecture migration {}", full.display()))?;
            self.apply_sql_script(&sql).await
                .with_context(|| format!("Application migration {}", fname))?;
            sqlx::query("INSERT INTO schema_migrations (filename) VALUES (?)")
                .bind(&fname)
                .execute(&self.pool)
                .await?;
            info!("✅ Migration appliquée: {}", fname);
        }
        Ok(())
    }

    /// Applique un script SQL multi-statements en le scindant sur ';' (simple, suffisant pour SQLite sans triggers)
    async fn apply_sql_script(&self, script: &str) -> Result<()> {
        // Retirer commentaires '-- ...' et lignes vides, puis splitter
        let mut current = String::new();
        for line in script.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if trimmed.starts_with("--") { continue; }
            current.push_str(line);
            current.push('\n');
        }
        for stmt in current.split(';') {
            let s = stmt.trim();
            if s.is_empty() { continue; }
            sqlx::query(s).execute(&self.pool).await?;
        }
        Ok(())
    }

    /// Sauvegarde la blockchain complète
    pub async fn save_blockchain(&self, blockchain: &Blockchain) -> Result<()> {
        info!("Sauvegarde de la blockchain (hauteur: {})", blockchain.height());

        let mut tx = self.pool.begin().await?;

        // Sauvegarder les nouveaux blocs
        for block in &blockchain.blocks {
            self.save_block_if_new(&mut tx, block).await?;
        }

        // Sauvegarder les comptes
        for account in blockchain.accounts.values() {
            self.save_account(&mut tx, account).await?;
        }

        // Sauvegarder les lois
        for law in blockchain.laws.values() {
            self.save_law(&mut tx, law).await?;
        }

        // Sauvegarder les votes
        for votes in blockchain.votes.values() {
            for vote in votes {
                self.save_vote(&mut tx, vote).await?;
            }
        }

        tx.commit().await?;
        info!("Blockchain sauvegardée avec succès");
        Ok(())
    }

    /// Charge la blockchain depuis la base de données
    pub async fn load_blockchain(&self) -> Result<Option<Blockchain>> {
        info!("Chargement de la blockchain depuis la base de données");

        // Vérifier s'il y a des blocs
        let block_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blocks")
            .fetch_one(&self.pool)
            .await?;

        if block_count == 0 {
            info!("Aucun bloc trouvé en base");
            return Ok(None);
        }

        let mut blockchain = Blockchain::new();
        blockchain.blocks.clear(); // Retirer le bloc genesis par défaut

        // Charger les blocs dans l'ordre
        let rows = sqlx::query("SELECT block_data FROM blocks ORDER BY block_number")
            .fetch_all(&self.pool)
            .await?;

        for row in rows {
            let block_data: String = row.get("block_data");
            let block: Block = serde_json::from_str(&block_data)
                .context("Erreur désérialisation bloc")?;
            blockchain.blocks.push(block);
        }

        // Rejouer les transactions pour reconstruire l'état (comptes, lois, votes, propositions, supporters)
        blockchain
            .rebuild_state_from_blocks()
            .context("Erreur lors de la reconstruction de l'état de la blockchain")?;

        // Charger les comptes
        let rows = sqlx::query("SELECT id, public_key, reputation, is_active, metadata FROM accounts")
            .fetch_all(&self.pool)
            .await?;

        for row in rows {
            let id: String = row.get("id");
            let account_id = uuid::Uuid::parse_str(&id)?;
            // TODO: Reconstruire l'objet Account depuis les données
        }

        info!("Blockchain chargée: {} blocs", blockchain.blocks.len());
        Ok(Some(blockchain))
    }

    /// Sauvegarde un bloc s'il n'existe pas déjà
    async fn save_block_if_new(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, block: &Block) -> Result<()> {
        let block_data = serde_json::to_string(block)?;
        
        sqlx::query(r#"
            INSERT OR IGNORE INTO blocks 
            (block_number, hash, previous_hash, timestamp, block_data)
            VALUES (?, ?, ?, ?, ?)
        "#)
        .bind(block.header.block_number as i64)
        .bind(block.hash.to_hex())
        .bind(block.header.previous_hash.to_hex())
        .bind(block.header.timestamp.to_rfc3339())
        .bind(block_data)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Sauvegarde un compte
    async fn save_account(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, account: &Account) -> Result<()> {
        let metadata = serde_json::to_string(&account.metadata)?;
        
        sqlx::query(r#"
            INSERT OR REPLACE INTO accounts 
            (id, public_key, reputation, is_active, metadata, updated_at)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        "#)
        .bind(account.id.to_string())
        .bind(account.public_key.to_hex())
        .bind(account.reputation as i64)
        .bind(account.is_active)
        .bind(metadata)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Sauvegarde une loi
    async fn save_law(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, law: &Law) -> Result<()> {
        sqlx::query(r#"
            INSERT OR REPLACE INTO laws 
            (id, title, content, summary, category, status, author, version, parent_law_id, content_hash, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
        "#)
        .bind(law.id.to_string())
        .bind(&law.title)
        .bind(&law.content)
        .bind(&law.summary)
        .bind(&law.category)
        .bind(format!("{:?}", law.status))
        .bind(law.author.to_hex())
        .bind(law.version as i64)
        .bind(law.parent_law_id.map(|id| id.to_string()))
        .bind(law.content_hash.to_hex())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Sauvegarde un vote
    async fn save_vote(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, vote: &Vote) -> Result<()> {
        sqlx::query(r#"
            INSERT OR REPLACE INTO votes 
            (id, law_id, voter, vote_type, weight, timestamp, signature, comment)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(vote.id.to_string())
        .bind(vote.law_id.to_string())
        .bind(vote.voter.to_hex())
        .bind(format!("{:?}", vote.vote_type))
        .bind(vote.weight)
        .bind(vote.timestamp.to_rfc3339())
        .bind(vote.signature.to_hex())
        .bind(&vote.comment)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Insère un engagement d'identité (commitment) de façon idempotente.
    /// Retourne (id, existed) où `existed=true` signifie que le hash était déjà présent.
    pub async fn insert_identity_commitment(
        &self,
        public_key: &str,
        did: &str,
        commitment_hash: &str,
        issuer_did: &str,
        issued_at: &str,
        expires_at: Option<&str>,
    ) -> Result<(i64, bool)> {
        let mut tx = self.pool.begin().await?;
        let insert_res = sqlx::query(r#"
            INSERT INTO identity_commitments (public_key, did, commitment_hash, issuer_did, issued_at, expires_at)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
            .bind(public_key)
            .bind(did)
            .bind(commitment_hash)
            .bind(issuer_did)
            .bind(issued_at)
            .bind(expires_at)
            .execute(&mut *tx).await;

        match insert_res {
            Ok(res) => {
                tx.commit().await?;
                let id = res.last_insert_rowid();
                Ok((id, false))
            }
            Err(e) => {
                // Conflit unicité: récupérer l'existant
                if let sqlx::Error::Database(db_err) = &e {
                    let msg = db_err.message();
                    if msg.contains("UNIQUE") && msg.contains("commitment_hash") {
                        let row = sqlx::query("SELECT id FROM identity_commitments WHERE commitment_hash = ?")
                            .bind(commitment_hash)
                            .fetch_one(&self.pool).await?;
                        let id: i64 = row.get("id");
                        return Ok((id, true));
                    }
                }
                Err(e.into())
            }
        }
    }

    /// Récupère un enregistrement identity_commitments par hash (lecture publique)
    pub async fn get_identity_commitment_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<(String, String, Option<String>, Option<String>, String)>> {
        let row = sqlx::query(
            r#"SELECT did, issuer_did, issued_at, expires_at, status FROM identity_commitments WHERE commitment_hash = ?"#
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(r) = row {
            let did: String = r.get("did");
            let issuer_did: String = r.get("issuer_did");
            let issued_at: Option<String> = r.try_get("issued_at").ok();
            let expires_at: Option<String> = r.try_get("expires_at").ok();
            let status: String = r.get("status");
            Ok(Some((did, issuer_did, issued_at, expires_at, status)))
        } else {
            Ok(None)
        }
    }
}
