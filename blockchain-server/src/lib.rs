//! blockchain-server — HTTP / P2P node implementation used in tests and local runs.
//!
//! This crate contains the server implementation that hosts the API, manages
//! the local blockchain state, storage, P2P networking and consensus helpers.
//! The library facade exposes a few convenient re-exports used by integration
//! tests and other workspace crates.
//!
//! Public re-exports:
//! - `ServerConfig` — server configuration loader and helpers
//! - `BlockchainNode` — main node type providing node operations (submit tx, mine, etc.)
//! - `create_api_router` — build an Axum router for embedding the HTTP API
//! - `ensure_minimal_schema` — helper to initialize minimal DB schema for tests
//!
//! Example (start a node and get stats):
//!
//! ```no_run
//! use blockchain_server::{ServerConfig, BlockchainNode};
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let cfg = ServerConfig::default();
//!     let node = BlockchainNode::new(cfg).await?;
//!     println!("node id: {}", node.get_node_id());
//!     Ok(())
//! }
//! ```

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
pub mod identity_root;
#[cfg(feature = "zkp_groth16")]
pub mod zkp_verifier;
pub mod dbutil;

// Re-exports for convenient access in tests
pub use config::ServerConfig;
pub use node::BlockchainNode;
pub use api::create_api_router;
pub use dbutil::ensure_minimal_schema;