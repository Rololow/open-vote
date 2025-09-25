// Library facade for blockchain-server to support integration tests

pub mod database;
pub mod migration;
pub mod security;
pub mod node;
pub mod api;
pub mod network;
pub mod consensus;
pub mod config;
pub mod storage;
pub mod issuer;

// Re-exports for convenient access in tests
pub use config::ServerConfig;
pub use node::BlockchainNode;
pub use api::create_api_router;