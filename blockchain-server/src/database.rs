use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use uuid::Uuid;

/// Module de gestion de la base de données SQLite
/// Remplace les données en mémoire par une persistance réelle

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbLaw {
    pub id: String,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub category: String,
    pub status: String,
    pub author: String,
    pub created_at: String,
    pub votes_for: i32,
    pub votes_against: i32,
    pub votes_abstain: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbAccount {
    pub id: String,
    pub public_key: String,
    pub display_name: String,
    pub reputation: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbVote {
    pub id: String,
    pub law_id: String,
    pub voter_id: String,
    pub vote_type: String,
    pub timestamp: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbBlock {
    pub id: String,
    pub block_number: i64,
    pub hash: String,
    pub previous_hash: String,
    pub merkle_root: String,
    pub nonce: i64,
    pub difficulty: i32,
    pub timestamp: String,
    pub transaction_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DbTransaction {
    pub id: String,
    pub block_id: String,
    pub from_address: String,
    pub to_address: String,
    pub transaction_type: String,
    pub data: String,
    pub signature: String,
    pub timestamp: String,
}

pub struct Database {
    pub pool: Pool<Sqlite>,
}

impl Database {
    /// Initialise la connexion à la base de données
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("🗄️ Connexion à la base de données: {}", database_url);
        
        // Ensure parent directory exists for file-based SQLite URLs
        if let Some(path_str) = database_url.strip_prefix("sqlite:") {
            let trimmed = path_str.trim_start_matches('/');
            if !trimmed.starts_with(':') && !trimmed.starts_with("memory") {
                let file_path = if path_str.starts_with("//") { &path_str[2..] } else { path_str };
                let file_path = file_path.split('?').next().unwrap_or(file_path);
                let file_path = file_path.trim_start_matches('/');
                let p = Path::new(file_path);
                if let Some(dir) = p.parent() {
                    if !dir.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(dir).await?;
                    }
                }
                if !p.exists() {
                    use tokio::io::AsyncWriteExt;
                    if let Some(dir) = p.parent() {
                        tokio::fs::create_dir_all(dir).await.ok();
                    }
                    let mut f = tokio::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(p)
                        .await?;
                    f.flush().await.ok();
                }
            }
        }

        let opts = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Delete)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(opts)
            .await?;

        let db = Self { pool };
        
        // Créer les tables si elles n'existent pas
        db.create_tables().await?;
        
    // Pas de données de démonstration – la base démarre vide
        
        info!("✅ Base de données initialisée avec succès");
        Ok(db)
    }

    /// Crée les tables nécessaires
    async fn create_tables(&self) -> Result<()> {
        info!("📋 Création des tables de base de données...");

        // Table des lois
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS laws (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                summary TEXT NOT NULL,
                category TEXT NOT NULL,
                status TEXT NOT NULL,
                author TEXT NOT NULL,
                created_at TEXT NOT NULL,
                votes_for INTEGER DEFAULT 0,
                votes_against INTEGER DEFAULT 0,
                votes_abstain INTEGER DEFAULT 0
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Table des comptes
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                public_key TEXT UNIQUE NOT NULL,
                display_name TEXT NOT NULL,
                reputation INTEGER DEFAULT 50,
                created_at TEXT NOT NULL
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Table des votes
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS votes (
                id TEXT PRIMARY KEY,
                law_id TEXT NOT NULL,
                voter_id TEXT NOT NULL,
                vote_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                comment TEXT,
                FOREIGN KEY (law_id) REFERENCES laws (id),
                FOREIGN KEY (voter_id) REFERENCES accounts (id),
                UNIQUE(law_id, voter_id)
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Table des blocs
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS blocks (
                id TEXT PRIMARY KEY,
                block_number INTEGER UNIQUE NOT NULL,
                hash TEXT UNIQUE NOT NULL,
                previous_hash TEXT NOT NULL,
                merkle_root TEXT NOT NULL,
                nonce INTEGER NOT NULL,
                difficulty INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                transaction_count INTEGER DEFAULT 0
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Pas de données de démonstration – la base démarre vide
        Ok(())
    }

    /// Récupère toutes les lois
    pub async fn get_all_laws(&self) -> Result<Vec<DbLaw>> {
        let laws = sqlx::query_as::<_, DbLaw>("SELECT * FROM laws ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        
        info!("📜 Récupération de {} lois", laws.len());
        Ok(laws)
    }

    /// Récupère une loi par ID
    pub async fn get_law_by_id(&self, id: &str) -> Result<Option<DbLaw>> {
        let law = sqlx::query_as::<_, DbLaw>("SELECT * FROM laws WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        
        Ok(law)
    }

    /// Récupère tous les comptes
    pub async fn get_all_accounts(&self) -> Result<Vec<DbAccount>> {
        let accounts = sqlx::query_as::<_, DbAccount>("SELECT * FROM accounts ORDER BY reputation DESC")
            .fetch_all(&self.pool)
            .await?;
        
        info!("👥 Récupération de {} comptes", accounts.len());
        Ok(accounts)
    }

    /// Récupère les votes pour une loi
    pub async fn get_votes_for_law(&self, law_id: &str) -> Result<Vec<DbVote>> {
        let votes = sqlx::query_as::<_, DbVote>("SELECT * FROM votes WHERE law_id = ? ORDER BY timestamp DESC")
            .bind(law_id)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(votes)
    }

    /// Enregistre un nouveau vote
    pub async fn create_vote(&self, law_id: &str, voter_id: &str, vote_type: &str, comment: Option<&str>) -> Result<DbVote> {
        let vote_id = Uuid::new_v4().to_string();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        let timestamp_str = format!("2025-09-18T{:02}:{:02}:{:02}Z", 
            (timestamp % 86400) / 3600, 
            (timestamp % 3600) / 60, 
            timestamp % 60);

        // Insérer le vote
        sqlx::query(r#"
            INSERT OR REPLACE INTO votes (id, law_id, voter_id, vote_type, timestamp, comment)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&vote_id)
        .bind(law_id)
        .bind(voter_id)
        .bind(vote_type)
        .bind(&timestamp_str)
        .bind(comment)
        .execute(&self.pool)
        .await?;

        // Mettre à jour les compteurs de la loi
        match vote_type {
            "for" => {
                sqlx::query("UPDATE laws SET votes_for = votes_for + 1 WHERE id = ?")
                    .bind(law_id)
                    .execute(&self.pool)
                    .await?;
            },
            "against" => {
                sqlx::query("UPDATE laws SET votes_against = votes_against + 1 WHERE id = ?")
                    .bind(law_id)
                    .execute(&self.pool)
                    .await?;
            },
            "abstain" => {
                sqlx::query("UPDATE laws SET votes_abstain = votes_abstain + 1 WHERE id = ?")
                    .bind(law_id)
                    .execute(&self.pool)
                    .await?;
            },
            _ => {}
        }

        let vote = DbVote {
            id: vote_id,
            law_id: law_id.to_string(),
            voter_id: voter_id.to_string(),
            vote_type: vote_type.to_string(),
            timestamp: timestamp_str,
            comment: comment.map(|s| s.to_string()),
        };

        info!("🗳️ Nouveau vote enregistré: {} pour la loi {}", vote_type, law_id);
        Ok(vote)
    }

    /// Récupère les statistiques générales
    pub async fn get_statistics(&self) -> Result<DatabaseStats> {
        let total_laws: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM laws")
            .fetch_one(&self.pool)
            .await?;

        let total_accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
            .fetch_one(&self.pool)
            .await?;

        let total_votes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM votes")
            .fetch_one(&self.pool)
            .await?;

        let active_laws: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM laws WHERE status = 'Active'")
            .fetch_one(&self.pool)
            .await?;

        let voting_laws: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM laws WHERE status = 'Voting'")
            .fetch_one(&self.pool)
            .await?;

        Ok(DatabaseStats {
            total_laws: total_laws as u32,
            total_accounts: total_accounts as u32,
            total_votes: total_votes as u32,
            active_laws: active_laws as u32,
            voting_laws: voting_laws as u32,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub total_laws: u32,
    pub total_accounts: u32,
    pub total_votes: u32,
    pub active_laws: u32,
    pub voting_laws: u32,
}