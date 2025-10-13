use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use common::identity::zkp_prelude::{Commitment, MerkleAccumulator, merkle_proof_for, verify_merkle_proof, IdentityAccumulator};

fn parse_hex32(line: &str) -> Option<Commitment> {
    let s = line.trim();
    if s.len() != 64 { return None; }
    let mut out = [0u8; 32];
    hex::decode_to_slice(s, &mut out).ok()?;
    Some(out)
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let log_path: PathBuf = args.next().map(Into::into)
        .unwrap_or_else(|| PathBuf::from("blockchain-server/data/commitments.log"));
    let mode = args.next().unwrap_or_else(|| "root".to_string());

    let content = fs::read_to_string(&log_path)
        .with_context(|| format!("read {}", log_path.display()))?;
    let mut leaves: Vec<Commitment> = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.trim().is_empty() { continue; }
        match parse_hex32(line) {
            Some(h) => leaves.push(h),
            None => anyhow::bail!("invalid hex hash at line {}", idx+1),
        }
    }

    match mode.as_str() {
        "root" => {
            let mut acc = MerkleAccumulator::new();
            for l in &leaves { IdentityAccumulator::append(&mut acc, *l)?; }
            println!("root={}", hex::encode(IdentityAccumulator::root(&acc)));
        }
        "prove" => {
            let idx_str = args.next().context("missing leaf index for prove mode")?;
            let idx: usize = idx_str.parse().context("index parse")?;
            let proof = merkle_proof_for(&leaves, idx).context("no proof for index")?;
            // print json proof with siblings hex
            let siblings_hex: Vec<String> = proof.siblings.iter().map(|h| hex::encode(h)).collect();
            let json = serde_json::json!({
                "leaf_index": proof.leaf_index,
                "siblings": siblings_hex,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        "verify" => {
            let idx_str = args.next().context("missing leaf index for verify mode")?;
            let idx: usize = idx_str.parse().context("index parse")?;
            let proof_path: PathBuf = args.next().context("missing proof.json path")?.into();
            let proof_json = fs::read_to_string(&proof_path).with_context(|| format!("read {}", proof_path.display()))?;
            let v: serde_json::Value = serde_json::from_str(&proof_json)?;
            let leaf_index = v.get("leaf_index").and_then(|x| x.as_u64()).context("leaf_index")? as usize;
            if leaf_index != idx { anyhow::bail!("leaf_index mismatch: proof={}, arg={}", leaf_index, idx); }
            let siblings = v.get("siblings").and_then(|x| x.as_array()).context("siblings")?;
            let mut sibs: Vec<Commitment> = Vec::with_capacity(siblings.len());
            for s in siblings {
                let hexs = s.as_str().context("sibling hex")?;
                let mut arr = [0u8; 32];
                hex::decode_to_slice(hexs, &mut arr).context("sibling decode")?;
                sibs.push(arr);
            }
            let proof = common::identity::zkp_prelude::MerkleProof { siblings: sibs, leaf_index };
            let mut acc = MerkleAccumulator::new();
            for l in &leaves { IdentityAccumulator::append(&mut acc, *l)?; }
            let root = IdentityAccumulator::root(&acc);
            let leaf = leaves.get(idx).context("leaf index out of bounds")?;
            let ok = verify_merkle_proof(leaf, &proof, &root);
            println!("valid={}", ok);
            if !ok { anyhow::bail!("invalid proof"); }
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  commitment_proof [<commitments.log>] root");
            eprintln!("  commitment_proof [<commitments.log>] prove <leaf_index>");
            eprintln!("  commitment_proof [<commitments.log>] verify <leaf_index> <proof.json>");
            anyhow::bail!("unknown mode: {}", mode);
        }
    }
    Ok(())
}
