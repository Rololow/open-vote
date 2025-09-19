use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info, warn};

use crate::database::{Database, DbLaw, DbAccount, DbVote};

/// Module de migration des données depuis le serveur Python mock
/// Permet d'importer les lois, comptes et votes existants

#[derive(Debug, Deserialize)]
pub struct MockLaw {
    pub id: String,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub category: String,
    pub status: String,
    pub author: String,
    pub created_at: String,
    pub votes_for: Option<i32>,
    pub votes_against: Option<i32>,
    pub votes_abstain: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct MockAccount {
    pub id: String,
    pub public_key: String,
    pub display_name: String,
    pub reputation: Option<i32>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct MockVote {
    pub id: String,
    pub law_id: String,
    pub voter_id: String,
    pub vote_type: String,
    pub timestamp: String,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MigrationReport {
    pub success: bool,
    pub laws_migrated: u32,
    pub accounts_migrated: u32,
    pub votes_migrated: u32,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

pub struct DataMigrator {
    client: Client,
    mock_server_url: String,
}

impl DataMigrator {
    pub fn new(mock_server_url: &str) -> Self {
        Self {
            client: Client::new(),
            mock_server_url: mock_server_url.to_string(),
        }
    }

    /// Migre toutes les données depuis le serveur mock
    pub async fn migrate_all_data(&self, db: &Database) -> Result<MigrationReport> {
        let start_time = std::time::Instant::now();
        let mut report = MigrationReport {
            success: true,
            laws_migrated: 0,
            accounts_migrated: 0,
            votes_migrated: 0,
            errors: Vec::new(),
            duration_ms: 0,
        };

        info!("🚀 Début de la migration des données depuis {}", self.mock_server_url);

        // Étape 1: Migrer les comptes (doit être fait en premier à cause des contraintes FK)
        match self.migrate_accounts(db).await {
            Ok(count) => {
                report.accounts_migrated = count;
                info!("✅ {} comptes migrés avec succès", count);
            },
            Err(e) => {
                let error_msg = format!("Erreur migration comptes: {}", e);
                error!("{}", error_msg);
                report.errors.push(error_msg);
                report.success = false;
            }
        }

        // Étape 2: Migrer les lois
        match self.migrate_laws(db).await {
            Ok(count) => {
                report.laws_migrated = count;
                info!("✅ {} lois migrées avec succès", count);
            },
            Err(e) => {
                let error_msg = format!("Erreur migration lois: {}", e);
                error!("{}", error_msg);
                report.errors.push(error_msg);
                report.success = false;
            }
        }

        // Étape 3: Migrer les votes (doit être fait après lois et comptes)
        match self.migrate_votes(db).await {
            Ok(count) => {
                report.votes_migrated = count;
                info!("✅ {} votes migrés avec succès", count);
            },
            Err(e) => {
                let error_msg = format!("Erreur migration votes: {}", e);
                error!("{}", error_msg);
                report.errors.push(error_msg);
                report.success = false;
            }
        }

        report.duration_ms = start_time.elapsed().as_millis() as u64;
        
        if report.success {
            info!("🎉 Migration terminée avec succès en {}ms", report.duration_ms);
        } else {
            warn!("⚠️ Migration terminée avec erreurs en {}ms", report.duration_ms);
        }

        Ok(report)
    }

    /// Migre les comptes depuis le serveur mock
    async fn migrate_accounts(&self, db: &Database) -> Result<u32> {
        info!("👥 Migration des comptes...");
        
        let url = format!("{}/api/accounts", self.mock_server_url);
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Échec de la récupération des comptes")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Erreur HTTP {}: {}", response.status(), response.text().await?));
        }

        let json: Value = response.json().await
            .context("Échec du parsing JSON des comptes")?;

        let accounts: Vec<MockAccount> = if let Some(accounts_array) = json.get("accounts") {
            serde_json::from_value(accounts_array.clone())
                .context("Échec de la désérialisation des comptes")?
        } else {
            return Err(anyhow::anyhow!("Champ 'accounts' non trouvé dans la réponse"));
        };

        let mut migrated_count = 0;

        for mock_account in accounts {
            let db_account = DbAccount {
                id: mock_account.id,
                public_key: mock_account.public_key,
                display_name: mock_account.display_name,
                reputation: mock_account.reputation.unwrap_or(50),
                created_at: mock_account.created_at,
            };

            // Insérer le compte dans SQLite (ignorer si existe déjà)
            match sqlx::query(r#"
                INSERT OR IGNORE INTO accounts (id, public_key, display_name, reputation, created_at)
                VALUES (?, ?, ?, ?, ?)
            "#)
            .bind(&db_account.id)
            .bind(&db_account.public_key)
            .bind(&db_account.display_name)
            .bind(db_account.reputation)
            .bind(&db_account.created_at)
            .execute(&db.pool)
            .await
            {
                Ok(result) => {
                    if result.rows_affected() > 0 {
                        migrated_count += 1;
                    }
                },
                Err(e) => {
                    warn!("Échec insertion compte {}: {}", db_account.id, e);
                }
            }
        }

        Ok(migrated_count)
    }

    /// Migre les lois depuis le serveur mock
    async fn migrate_laws(&self, db: &Database) -> Result<u32> {
        info!("📜 Migration des lois...");
        
        let url = format!("{}/api/laws", self.mock_server_url);
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Échec de la récupération des lois")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Erreur HTTP {}: {}", response.status(), response.text().await?));
        }

        let json: Value = response.json().await
            .context("Échec du parsing JSON des lois")?;

        let laws: Vec<MockLaw> = if let Some(laws_array) = json.get("laws") {
            serde_json::from_value(laws_array.clone())
                .context("Échec de la désérialisation des lois")?
        } else {
            return Err(anyhow::anyhow!("Champ 'laws' non trouvé dans la réponse"));
        };

        let mut migrated_count = 0;

        for mock_law in laws {
            let db_law = DbLaw {
                id: mock_law.id,
                title: mock_law.title,
                content: mock_law.content,
                summary: mock_law.summary,
                category: mock_law.category,
                status: mock_law.status,
                author: mock_law.author,
                created_at: mock_law.created_at,
                votes_for: mock_law.votes_for.unwrap_or(0),
                votes_against: mock_law.votes_against.unwrap_or(0),
                votes_abstain: mock_law.votes_abstain.unwrap_or(0),
            };

            // Insérer la loi dans SQLite (ignorer si existe déjà)
            match sqlx::query(r#"
                INSERT OR IGNORE INTO laws (id, title, content, summary, category, status, author, created_at, votes_for, votes_against, votes_abstain)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&db_law.id)
            .bind(&db_law.title)
            .bind(&db_law.content)
            .bind(&db_law.summary)
            .bind(&db_law.category)
            .bind(&db_law.status)
            .bind(&db_law.author)
            .bind(&db_law.created_at)
            .bind(db_law.votes_for)
            .bind(db_law.votes_against)
            .bind(db_law.votes_abstain)
            .execute(&db.pool)
            .await
            {
                Ok(result) => {
                    if result.rows_affected() > 0 {
                        migrated_count += 1;
                    }
                },
                Err(e) => {
                    warn!("Échec insertion loi {}: {}", db_law.id, e);
                }
            }
        }

        Ok(migrated_count)
    }

    /// Migre les votes depuis le serveur mock
    async fn migrate_votes(&self, db: &Database) -> Result<u32> {
        info!("🗳️ Migration des votes...");
        
        // Récupérer les votes pour chaque loi
        let laws = db.get_all_laws().await
            .context("Impossible de récupérer les lois pour la migration des votes")?;

        let mut migrated_count = 0;

        for law in laws {
            let url = format!("{}/api/laws/{}/votes", self.mock_server_url, law.id);
            
            match self.client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    match response.json::<Value>().await {
                        Ok(json) => {
                            if let Some(votes_array) = json.get("votes") {
                                match serde_json::from_value::<Vec<MockVote>>(votes_array.clone()) {
                                    Ok(votes) => {
                                        for mock_vote in votes {
                                            let db_vote = DbVote {
                                                id: mock_vote.id,
                                                law_id: mock_vote.law_id,
                                                voter_id: mock_vote.voter_id,
                                                vote_type: mock_vote.vote_type,
                                                timestamp: mock_vote.timestamp,
                                                comment: mock_vote.comment,
                                            };

                                            // Insérer le vote dans SQLite (ignorer si existe déjà)
                                            match sqlx::query(r#"
                                                INSERT OR IGNORE INTO votes (id, law_id, voter_id, vote_type, timestamp, comment)
                                                VALUES (?, ?, ?, ?, ?, ?)
                                            "#)
                                            .bind(&db_vote.id)
                                            .bind(&db_vote.law_id)
                                            .bind(&db_vote.voter_id)
                                            .bind(&db_vote.vote_type)
                                            .bind(&db_vote.timestamp)
                                            .bind(&db_vote.comment)
                                            .execute(&db.pool)
                                            .await
                                            {
                                                Ok(result) => {
                                                    if result.rows_affected() > 0 {
                                                        migrated_count += 1;
                                                    }
                                                },
                                                Err(e) => {
                                                    warn!("Échec insertion vote {}: {}", db_vote.id, e);
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => warn!("Échec désérialisation votes pour loi {}: {}", law.id, e)
                                }
                            }
                        },
                        Err(e) => warn!("Échec parsing JSON votes pour loi {}: {}", law.id, e)
                    }
                },
                Ok(response) => {
                    warn!("Erreur HTTP {} pour votes de la loi {}", response.status(), law.id);
                },
                Err(e) => {
                    warn!("Échec récupération votes pour loi {}: {}", law.id, e);
                }
            }
        }

        Ok(migrated_count)
    }

    /// Teste la connectivité avec le serveur mock
    pub async fn test_connection(&self) -> Result<bool> {
        info!("🔍 Test de connexion au serveur mock...");
        
        let url = format!("{}/health", self.mock_server_url);
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                info!("✅ Connexion au serveur mock réussie");
                Ok(true)
            },
            Ok(response) => {
                warn!("⚠️ Serveur mock répond avec le statut: {}", response.status());
                Ok(false)
            },
            Err(e) => {
                error!("❌ Impossible de se connecter au serveur mock: {}", e);
                Ok(false)
            }
        }
    }
}