use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};
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
        
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        let db = Self { pool };
        
        // Créer les tables si elles n'existent pas
        db.create_tables().await?;
        
        // Insérer des données de test si la base est vide
        db.seed_data().await?;
        
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

        // Table des transactions
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id TEXT PRIMARY KEY,
                block_id TEXT,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                data TEXT NOT NULL,
                signature TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (block_id) REFERENCES blocks (id)
            )
        "#)
        .execute(&self.pool)
        .await?;

        info!("✅ Tables créées avec succès");
        Ok(())
    }

    /// Insère des données de test
    async fn seed_data(&self) -> Result<()> {
        // Vérifier si des données existent déjà
        let law_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM laws")
            .fetch_one(&self.pool)
            .await?;

        if law_count > 0 {
            info!("📊 Données existantes trouvées, pas de seed nécessaire");
            return Ok(());
        }

        info!("🌱 Insertion des données de test...");

        // Insérer des comptes de test
        let accounts = vec![
            DbAccount {
                id: "acc-1".to_string(),
                public_key: "0x1234567890abcdef".to_string(),
                display_name: "Marie Dupont".to_string(),
                reputation: 85,
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            DbAccount {
                id: "acc-2".to_string(),
                public_key: "0xabcdef1234567890".to_string(),
                display_name: "Jean Martin".to_string(),
                reputation: 92,
                created_at: "2024-01-15T00:00:00Z".to_string(),
            },
            DbAccount {
                id: "acc-3".to_string(),
                public_key: "0x5678901234abcdef".to_string(),
                display_name: "Sophie Dubois".to_string(),
                reputation: 78,
                created_at: "2024-02-01T00:00:00Z".to_string(),
            },
        ];

        for account in &accounts {
            sqlx::query(r#"
                INSERT OR IGNORE INTO accounts (id, public_key, display_name, reputation, created_at)
                VALUES (?, ?, ?, ?, ?)
            "#)
            .bind(&account.id)
            .bind(&account.public_key)
            .bind(&account.display_name)
            .bind(account.reputation)
            .bind(&account.created_at)
            .execute(&self.pool)
            .await?;
        }

        // Insérer des lois de test
        let laws = vec![
            DbLaw {
                id: "law-1".to_string(),
                title: "Loi sur la Protection des Données Citoyennes".to_string(),
                content: "Cette loi vise à protéger les données personnelles des citoyens dans l'écosystème numérique gouvernemental. Elle établit des règles strictes pour la collecte, le traitement et le stockage des informations personnelles.".to_string(),
                summary: "Protection renforcée des données personnelles des citoyens".to_string(),
                category: "Numérique".to_string(),
                status: "Active".to_string(),
                author: "0x1234567890abcdef".to_string(),
                created_at: "2024-01-15T10:30:00Z".to_string(),
                votes_for: 156,
                votes_against: 23,
                votes_abstain: 12,
            },
            DbLaw {
                id: "law-2".to_string(),
                title: "Règlement sur la Transparence Budgétaire".to_string(),
                content: "Établit les règles de transparence pour la publication des budgets publics sur la blockchain. Tous les citoyens auront accès aux détails des dépenses publiques en temps réel.".to_string(),
                summary: "Transparence obligatoire des budgets publics".to_string(),
                category: "Finances".to_string(),
                status: "Voting".to_string(),
                author: "0xabcdef1234567890".to_string(),
                created_at: "2024-02-20T14:45:00Z".to_string(),
                votes_for: 89,
                votes_against: 45,
                votes_abstain: 8,
            },
            DbLaw {
                id: "law-3".to_string(),
                title: "Décret sur la Participation Citoyenne Numérique".to_string(),
                content: "Définit les modalités de participation des citoyens aux décisions publiques via la plateforme blockchain. Inclut les procédures de vote, de débat et de proposition de nouvelles lois.".to_string(),
                summary: "Cadre pour la participation citoyenne en ligne".to_string(),
                category: "Gouvernance".to_string(),
                status: "InReview".to_string(),
                author: "0x5678901234abcdef".to_string(),
                created_at: "2024-03-10T09:15:00Z".to_string(),
                votes_for: 34,
                votes_against: 12,
                votes_abstain: 5,
            },
        ];

        for law in &laws {
            sqlx::query(r#"
                INSERT OR IGNORE INTO laws (id, title, content, summary, category, status, author, created_at, votes_for, votes_against, votes_abstain)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&law.id)
            .bind(&law.title)
            .bind(&law.content)
            .bind(&law.summary)
            .bind(&law.category)
            .bind(&law.status)
            .bind(&law.author)
            .bind(&law.created_at)
            .bind(law.votes_for)
            .bind(law.votes_against)
            .bind(law.votes_abstain)
            .execute(&self.pool)
            .await?;
        }

        // Insérer des votes de test
        let votes = vec![
            DbVote {
                id: "vote-1".to_string(),
                law_id: "law-1".to_string(),
                voter_id: "acc-1".to_string(),
                vote_type: "for".to_string(),
                timestamp: "2024-01-16T10:30:00Z".to_string(),
                comment: Some("Essentiel pour la protection des citoyens".to_string()),
            },
            DbVote {
                id: "vote-2".to_string(),
                law_id: "law-1".to_string(),
                voter_id: "acc-2".to_string(),
                vote_type: "for".to_string(),
                timestamp: "2024-01-16T11:15:00Z".to_string(),
                comment: None,
            },
            DbVote {
                id: "vote-3".to_string(),
                law_id: "law-2".to_string(),
                voter_id: "acc-3".to_string(),
                vote_type: "against".to_string(),
                timestamp: "2024-02-21T09:00:00Z".to_string(),
                comment: Some("Mesures trop restrictives".to_string()),
            },
        ];

        for vote in &votes {
            sqlx::query(r#"
                INSERT OR IGNORE INTO votes (id, law_id, voter_id, vote_type, timestamp, comment)
                VALUES (?, ?, ?, ?, ?, ?)
            "#)
            .bind(&vote.id)
            .bind(&vote.law_id)
            .bind(&vote.voter_id)
            .bind(&vote.vote_type)
            .bind(&vote.timestamp)
            .bind(&vote.comment)
            .execute(&self.pool)
            .await?;
        }

        info!("✅ Données de test insérées avec succès");
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