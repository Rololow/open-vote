use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockchainStats {
    pub total_blocks: u64,
    pub total_transactions: u64,
    pub total_accounts: u64,
    pub active_proposals: u64,
    pub latest_block_time: Option<DateTime<Utc>>,
    pub network_peers: u32,
    pub mempool_size: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub number: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_count: usize,
    pub miner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub transaction_type: String,
    pub timestamp: DateTime<Utc>,
    pub sender: String,
    pub status: TransactionStatus,
    pub block_number: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub public_key: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub transaction_count: u32,
    pub is_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub support_count: u32,
    pub status: ProposalStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Promoted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkPeer {
    pub id: String,
    pub address: String,
    pub status: PeerStatus,
    pub last_seen: DateTime<Utc>,
    pub block_height: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Syncing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub status: ServerStatus,
    pub uptime: u64, // en secondes
    pub version: String,
    pub last_ping: DateTime<Utc>,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub connections: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerStatus {
    Online,
    Offline,
    Maintenance,
    Warning,
}