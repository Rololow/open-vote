use anyhow::{Result, Context};
use sqlx::{SqlitePool, Row};
use tracing::{info, error};
use serde_json;

use common::{Blockchain, Block, Transaction, Account, Law, Vote};

/// Gestionnaire de stockage pour la blockchain
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Crée une nouvelle instance de stockage
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connexion à la base de données: {}", database_url);
        
        let pool = SqlitePool::connect(database_url).await
            .context("Erreur connexion base de données")?;

        let storage = Self { pool };
        
        // Initialiser les tables
        storage.init_tables().await?;
        
        Ok(storage)
    }

    /// Initialise les tables de la base de données
    async fn init_tables(&self) -> Result<()> {
        info!("Initialisation des tables de la base de données");

        // Table des blocs
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                block_number INTEGER NOT NULL UNIQUE,
                hash TEXT NOT NULL UNIQUE,
                previous_hash TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                block_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#)
        .execute(&self.pool)
        .await
        .context("Erreur création table blocks")?;

        // Table des transactions
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id TEXT PRIMARY KEY,
                block_number INTEGER,
                transaction_type TEXT NOT NULL,
                sender TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                signature TEXT NOT NULL,
                transaction_data TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (block_number) REFERENCES blocks (block_number)
            )
        "#)
        .execute(&self.pool)
        .await
        .context("Erreur création table transactions")?;

        // Table des comptes
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                public_key TEXT NOT NULL UNIQUE,
                reputation INTEGER NOT NULL DEFAULT 0,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                metadata TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#)
        .execute(&self.pool)
        .await
        .context("Erreur création table accounts")?;

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
                version INTEGER NOT NULL DEFAULT 1,
                parent_law_id TEXT,
                content_hash TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#)
        .execute(&self.pool)
        .await
        .context("Erreur création table laws")?;

        // Table des votes
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS votes (
                id TEXT PRIMARY KEY,
                law_id TEXT NOT NULL,
                voter TEXT NOT NULL,
                vote_type TEXT NOT NULL,
                weight REAL NOT NULL,
                timestamp TEXT NOT NULL,
                signature TEXT NOT NULL,
                comment TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (law_id) REFERENCES laws (id),
                UNIQUE(law_id, voter)
            )
        "#)
        .execute(&self.pool)
        .await
        .context("Erreur création table votes")?;

        // Index pour les performances
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_number ON blocks (block_number)")
            .execute(&self.pool).await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_block ON transactions (block_number)")
            .execute(&self.pool).await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_votes_law ON votes (law_id)")
            .execute(&self.pool).await?;

        info!("Tables initialisées avec succès");
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
}
