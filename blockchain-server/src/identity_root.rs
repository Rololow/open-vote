use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use common::identity::zkp_prelude::{Commitment, MerkleAccumulator, IdentityAccumulator};
use chrono::Utc;
use serde::{Deserialize, Serialize};

fn parse_hex32(line: &str) -> Option<Commitment> {
    let s = line.trim();
    if s.len() != 64 { return None; }
    let mut out = [0u8; 32];
    hex::decode_to_slice(s, &mut out).ok()?;
    Some(out)
}

/// Read commitments.log (32-byte hex per non-empty line) and compute the Merkle root.
/// Returns the root and the number of leaves parsed.
pub fn compute_latest_root_from_file(path: &Path) -> Result<([u8;32], usize)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // No file yet: empty tree
            return Ok(([0u8;32], 0));
        }
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };
    let mut leaves: Vec<Commitment> = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() { continue; }
        match parse_hex32(t) {
            Some(h) => leaves.push(h),
            None => anyhow::bail!("invalid hex hash at line {}", idx + 1),
        }
    }
    let mut acc = MerkleAccumulator::new();
    for l in &leaves { IdentityAccumulator::append(&mut acc, *l)?; }
    Ok((IdentityAccumulator::root(&acc), leaves.len()))
}

/// Helper to locate the commitments.log inside a given data directory.
pub fn commitments_log_path(data_directory: &str) -> PathBuf {
    Path::new(data_directory).join("commitments.log")
}

/// JSONL entry representing an anchored identity commitments root at a given block height.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootAnchorEntry {
    pub block: u64,
    pub timestamp: String,
    pub root: String,
    pub leaves: usize,
}

/// Path of the identity roots JSONL anchor log.
pub fn identity_roots_log_path(data_directory: &str) -> PathBuf {
    Path::new(data_directory).join("identity_roots.jsonl")
}

/// Append a new root anchor entry to identity_roots.jsonl for the given block number.
pub fn append_root_anchor(data_directory: &str, block_number: u64) -> Result<()> {
    let commitments_path = commitments_log_path(data_directory);
    let (root, leaves) = compute_latest_root_from_file(&commitments_path)?;
    let entry = RootAnchorEntry {
        block: block_number,
        timestamp: Utc::now().to_rfc3339(),
        root: hex::encode(root),
        leaves,
    };
    let line = serde_json::to_string(&entry)? + "\n";
    let log_path = identity_roots_log_path(data_directory);
    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() { let _ = fs::create_dir_all(parent); }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&log_path)
        .with_context(|| format!("open {} for append", log_path.display()))?;
    f.write_all(line.as_bytes())
        .with_context(|| format!("append to {}", log_path.display()))?;
    Ok(())
}

/// Read the latest N root anchors (from the end of the JSONL file).
pub fn read_last_root_anchors(data_directory: &str, limit: usize) -> Result<Vec<RootAnchorEntry>> {
    let log_path = identity_roots_log_path(data_directory);
    let content = match fs::read_to_string(&log_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("read {}", log_path.display())),
    };
    let mut entries = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() { continue; }
        match serde_json::from_str::<RootAnchorEntry>(t) {
            Ok(e) => entries.push(e),
            Err(_) => { /* skip malformed line */ }
        }
    }
    // Take last `limit` entries
    if entries.len() > limit {
        entries.drain(0..(entries.len() - limit));
    }
    Ok(entries)
}
