use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;
use anyhow::{Result, Context};
use uuid::Uuid;
use tracing::info;

/// Server configuration for a blockchain node.
///
/// Loads defaults, environment variables and optional JSON config file.
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
    /// List of allowed issuer DIDs (did:key:...), used by identity checks.
    #[serde(default)]
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
    /// Load the configuration from environment variables and optional files.
    pub async fn load() -> Result<Self> {
        let mut config = Self::default();

    // Load from environment variables
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

    // Support both BLOCKCHAIN_DATA_DIRECTORY (script uses this) and the older
    // BLOCKCHAIN_DATA_DIR env var for backward compatibility. Prefer the
    // more explicit BLOCKCHAIN_DATA_DIRECTORY when present.
        if let Ok(data_dir) = env::var("BLOCKCHAIN_DATA_DIRECTORY") {
            config.data_directory = data_dir;
        } else if let Ok(data_dir) = env::var("BLOCKCHAIN_DATA_DIR") {
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

    // Load from a configuration file if present
        let config_path = Path::new("blockchain-config.json");
        if config_path.exists() {
            let config_content = tokio::fs::read_to_string(config_path).await
                .context("Erreur lecture fichier de configuration")?;
            
            let file_config: ServerConfig = serde_json::from_str(&config_content)
                .context("Erreur parsing configuration JSON")?;
            
            // Fusionner avec la configuration par défaut
            config = file_config;
        }

        // Create the data directory if necessary
        tokio::fs::create_dir_all(&config.data_directory).await
            .context("Erreur création répertoire de données")?;

        Ok(config)
    }

    /// Load configuration from a specific JSON file.
    pub async fn load_from_file(path: &str) -> Result<Self> {
        info!("Loading configuration from: {}", path);
        
        let config_content = tokio::fs::read_to_string(path).await
            .context("Erreur lecture fichier de configuration")?;
        
    info!("Config file contents:");
    info!("{}", config_content);
        
        let config: ServerConfig = serde_json::from_str(&config_content)
            .context("Erreur parsing configuration JSON")?;
        
    info!("Configuration parsed successfully:");
    info!("   Node ID: {}", config.node_id);
    info!("   Bind address: {}", config.bind_address);
    info!("   API port: {}", config.api_port);
    info!("   P2P port: {}", config.p2p_port);
    info!("   Database URL: {}", config.database_url);
    info!("   Data directory: {}", config.data_directory);
        
        // Create the data directory if necessary
        tokio::fs::create_dir_all(&config.data_directory).await
            .context("Erreur création répertoire de données")?;
        
        info!("📂 Répertoire de données créé: {}", config.data_directory);
        
        Ok(config)
    }

    /// Save the configuration to a JSON file.
    pub async fn save(&self, path: &str) -> Result<()> {
        let config_json = serde_json::to_string_pretty(self)
            .context("Error serializing configuration")?;

        tokio::fs::write(path, config_json).await
            .context("Error writing configuration file")?;

        Ok(())
    }

    /// Return the full API URL.
    pub fn api_url(&self) -> String {
        format!("http://{}:{}", self.bind_address, self.api_port)
    }

    /// Return the P2P address (host:port).
    pub fn p2p_address(&self) -> String {
        format!("{}:{}", self.bind_address, self.p2p_port)
    }
}
