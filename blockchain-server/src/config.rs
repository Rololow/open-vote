use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use anyhow::{Result, Context};
use uuid::Uuid;
use tracing::info;

/// Configuration du serveur blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub node_id: String,
    pub bind_address: String,
    pub api_port: u16,
    pub p2p_port: u16,
    pub database_url: String,
    pub data_directory: String,
    pub max_connections: u32,
    pub block_time_seconds: u64,
    pub max_block_size: usize,
    pub enable_mining: bool,
    pub log_level: String,
    /// Liste des DID émetteurs autorisés (did:key:...), séparés par des virgules
    pub allowed_issuers_dids: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            bind_address: "0.0.0.0".to_string(),
            api_port: 8080,
            p2p_port: 8081,
            // Place the SQLite DB inside the data directory which we ensure exists
            // Use the sqlx-recommended file URL format
            database_url: "sqlite://./data/blockchain.db".to_string(),
            data_directory: "./data".to_string(),
            max_connections: 100,
            block_time_seconds: 300, // 5 minutes
            max_block_size: 1_000_000, // 1MB
            enable_mining: true,
            log_level: "info".to_string(),
            allowed_issuers_dids: Vec::new(),
        }
    }
}

impl ServerConfig {
    /// Charge la configuration depuis l'environnement et les fichiers
    pub async fn load() -> Result<Self> {
        let mut config = Self::default();

        // Charger depuis les variables d'environnement
        if let Ok(node_id) = env::var("BLOCKCHAIN_NODE_ID") {
            config.node_id = node_id;
        }

        if let Ok(bind_addr) = env::var("BLOCKCHAIN_BIND_ADDRESS") {
            config.bind_address = bind_addr;
        }

        if let Ok(api_port) = env::var("BLOCKCHAIN_API_PORT") {
            config.api_port = api_port.parse()
                .context("Port API invalide")?;
        }

        if let Ok(p2p_port) = env::var("BLOCKCHAIN_P2P_PORT") {
            config.p2p_port = p2p_port.parse()
                .context("Port P2P invalide")?;
        }

        if let Ok(db_url) = env::var("BLOCKCHAIN_DATABASE_URL") {
            config.database_url = db_url;
        }

        if let Ok(data_dir) = env::var("BLOCKCHAIN_DATA_DIR") {
            config.data_directory = data_dir;
        }

        if let Ok(enable_mining) = env::var("BLOCKCHAIN_ENABLE_MINING") {
            config.enable_mining = enable_mining.parse()
                .context("Valeur mining invalide")?;
        }

        // Liste d'émetteurs autorisés (séparés par des virgules)
        if let Ok(list) = env::var("ALLOWED_ISSUERS_DIDS") {
            let items = list
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            config.allowed_issuers_dids = items;
        }

        // Charger depuis le fichier de configuration si il existe
        let config_path = Path::new("blockchain-config.json");
        if config_path.exists() {
            let config_content = tokio::fs::read_to_string(config_path).await
                .context("Erreur lecture fichier de configuration")?;
            
            let file_config: ServerConfig = serde_json::from_str(&config_content)
                .context("Erreur parsing configuration JSON")?;
            
            // Fusionner avec la configuration par défaut
            config = file_config;
        }

        // Créer le répertoire de données si nécessaire
        tokio::fs::create_dir_all(&config.data_directory).await
            .context("Erreur création répertoire de données")?;

        Ok(config)
    }

    /// Charge la configuration depuis un fichier spécifique
    pub async fn load_from_file(path: &str) -> Result<Self> {
        info!("🔧 Chargement de la configuration depuis: {}", path);
        
        let config_content = tokio::fs::read_to_string(path).await
            .context("Erreur lecture fichier de configuration")?;
        
        info!("📄 Contenu du fichier de configuration:");
        info!("{}", config_content);
        
        let config: ServerConfig = serde_json::from_str(&config_content)
            .context("Erreur parsing configuration JSON")?;
        
        info!("✅ Configuration parsée avec succès:");
        info!("   🆔 Node ID: {}", config.node_id);
        info!("   🌐 Bind address: {}", config.bind_address);
        info!("   🔌 API port: {}", config.api_port);
        info!("   🔗 P2P port: {}", config.p2p_port);
        info!("   💾 Database URL: {}", config.database_url);
        info!("   📁 Data directory: {}", config.data_directory);
        
        // Créer le répertoire de données si nécessaire
        tokio::fs::create_dir_all(&config.data_directory).await
            .context("Erreur création répertoire de données")?;
        
        info!("📂 Répertoire de données créé: {}", config.data_directory);
        
        Ok(config)
    }

    /// Sauvegarde la configuration dans un fichier
    pub async fn save(&self, path: &str) -> Result<()> {
        let config_json = serde_json::to_string_pretty(self)
            .context("Erreur sérialisation configuration")?;
        
        tokio::fs::write(path, config_json).await
            .context("Erreur écriture fichier de configuration")?;
        
        Ok(())
    }

    /// Obtient l'URL complète de l'API
    pub fn api_url(&self) -> String {
        format!("http://{}:{}", self.bind_address, self.api_port)
    }

    /// Obtient l'adresse P2P
    pub fn p2p_address(&self) -> String {
        format!("{}:{}", self.bind_address, self.p2p_port)
    }
}
