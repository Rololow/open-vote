use gloo::net::http::Request;
use crate::types::*;
use serde_json;

#[derive(Clone)]
pub struct BlockchainService {
    host: String,      // ex: http://localhost
    ports: Vec<u16>,   // ex: [3000,3002,3004]
}

impl BlockchainService {
    /// Construction rétro-compatible (ancienne interface) avec un seul base_url (incluant un port)
    pub fn new(base_url: String) -> Self {
        // Tenter d'extraire le port final si présent
        let (host, port) = if let Some(idx) = base_url.rfind(':') {
            let trailing = &base_url[idx+1..];
            if trailing.chars().all(|c| c.is_ascii_digit()) {
                (base_url[..idx].to_string(), trailing.parse().unwrap_or(3000))
            } else {
                (base_url, 3000)
            }
        } else {
            (base_url, 3000)
        };
        Self { host, ports: vec![port] }
    }

    /// Nouveau constructeur multi-ports
    pub fn new_multi(host: String, ports: Vec<u16>) -> Self {
        let ports = if ports.is_empty() { vec![3000] } else { ports };
        Self { host, ports }
    }

    fn primary_base(&self) -> String { format!("{}:{}", self.host, self.ports[0]) }

    pub async fn get_stats(&self) -> Result<BlockchainStats, String> {
    let url = format!("{}/stats", self.primary_base());
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {}", e))?;

        if response.ok() {
            let stats: BlockchainStats = response
                .json()
                .await
                .map_err(|e| format!("Erreur de parsing JSON: {}", e))?;
            Ok(stats)
        } else {
            Err(format!("Erreur HTTP: {}", response.status()))
        }
    }

    pub async fn get_latest_blocks(&self, limit: usize) -> Result<Vec<Block>, String> {
    let url = format!("{}/api/blocks?limit={}", self.primary_base(), limit);
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {}", e))?;

        if response.ok() {
            let response_data: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Erreur de parsing JSON: {}", e))?;
            
            let blocks: Vec<Block> = response_data["blocks"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|block| Block {
                    number: block["number"].as_u64().unwrap_or(0),
                    hash: block["hash"].as_str().unwrap_or("").to_string(),
                    previous_hash: block["previous_hash"].as_str().unwrap_or("").to_string(),
                    timestamp: chrono::Utc::now(), // TODO: Parse real timestamp
                    transaction_count: block["transaction_count"].as_u64().unwrap_or(0) as usize,
                    miner: Some("Node-1".to_string()),
                })
                .collect();
                
            Ok(blocks)
        } else {
            Err(format!("Erreur HTTP: {}", response.status()))
        }
    }

    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<Transaction>, String> {
    let url = format!("{}/api/transactions?limit={}", self.primary_base(), limit);
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {}", e))?;

        if response.ok() {
            let response_data: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Erreur de parsing JSON: {}", e))?;
            
            let transactions: Vec<Transaction> = response_data["transactions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|tx| Transaction {
                    id: tx["id"].as_str().unwrap_or("").to_string(),
                    transaction_type: tx["transaction_type"].as_str().unwrap_or("Unknown").to_string(),
                    timestamp: chrono::Utc::now(), // TODO: Parse real timestamp from tx["timestamp"]
                    sender: tx["sender"].as_str().unwrap_or("").to_string(),
                    status: match tx["status"].as_str().unwrap_or("pending") {
                        "confirmed" => TransactionStatus::Confirmed,
                        "failed" => TransactionStatus::Failed,
                        _ => TransactionStatus::Pending,
                    },
                    block_number: tx["block_number"].as_u64(),
                })
                .collect();
                
            Ok(transactions)
        } else {
            Err(format!("Erreur HTTP: {}", response.status()))
        }
    }

    pub async fn get_network_peers(&self) -> Result<Vec<NetworkPeer>, String> {
        // Utiliser l'endpoint étendu s'il est disponible sinon fallback
        let extended_url = format!("{}/api/peers/extended", self.primary_base());
        let simple_url = format!("{}/api/peers", self.primary_base());
        let mut response = Request::get(&extended_url).send().await;
        if response.is_err() { response = Request::get(&simple_url).send().await; }
        let response = response.map_err(|e| format!("Erreur réseau: {}", e))?;

        if !response.ok() { return Err(format!("Erreur HTTP: {}", response.status())); }
        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Erreur de parsing JSON: {}", e))?;

        let peers_array = response_data["peers"].as_array().cloned().unwrap_or_default();
    let peers: Vec<NetworkPeer> = peers_array.iter().map(|peer| {
            let addr = peer["address"].as_str().unwrap_or("").to_string();
            NetworkPeer {
                id: peer["id"].as_str().unwrap_or_else(|| addr.as_str()).to_string(),
                address: addr,
                status: PeerStatus::Connected,
                last_seen: chrono::Utc::now(),
                block_height: peer["block_height"].as_u64().unwrap_or(0),
            }
        }).collect();
        Ok(peers)
    }

    pub async fn get_health(&self) -> Result<serde_json::Value, String> {
    let url = format!("{}/health", self.primary_base());
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {}", e))?;

        if response.ok() {
            let health: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Erreur de parsing JSON: {}", e))?;
            Ok(health)
        } else {
            Err(format!("Service indisponible: {}", response.status()))
        }
    }

    pub async fn get_active_servers(&self) -> Result<Vec<ServerInfo>, String> {
        let mut servers = Vec::new();
        // Extraire IP/host "nu" pour affichage (retirer schéma éventuel)
        let display_host = self.host.trim_end_matches('/').replace("http://", "").replace("https://", "");

        // 1. Ajouter les ports locaux configurés
        for port in &self.ports {
            let base = format!("{}:{}", self.host, port);
            let health_url = format!("{}/health", base);
            let status = match Request::get(&health_url).send().await { Ok(resp) if resp.ok() => ServerStatus::Online, _ => ServerStatus::Offline };
            servers.push(ServerInfo { name: format!("Local Node {}", port), ip: display_host.clone(), port: *port, status, uptime: 0, version: "1.0.0".to_string(), last_ping: chrono::Utc::now(), cpu_usage: None, memory_usage: None, connections: 0 });
        }

        // 2. Découvrir d'autres ports via peers (si fournissent address:port)
        if let Ok(peers) = self.get_network_peers().await {
            for p in peers {
                // Extraire port si possible
                if let Some(idx) = p.address.rfind(':') { if let Ok(port) = p.address[idx+1..].parse::<u16>() {
                    if !servers.iter().any(|s| s.port == port) {
                        let host_only = p.address[..idx].to_string();
                        let health_url = format!("http://{}/health", p.address.trim_start_matches("http://"));
                        let status = match Request::get(&health_url).send().await { Ok(resp) if resp.ok() => ServerStatus::Online, _ => ServerStatus::Offline };
                        servers.push(ServerInfo { name: format!("Peer {}", port), ip: host_only, port, status, uptime: 0, version: "1.0.0".to_string(), last_ping: chrono::Utc::now(), cpu_usage: None, memory_usage: None, connections: 0 });
                    }
                }}
            }
        }
        Ok(servers)
    }
}