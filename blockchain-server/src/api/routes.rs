use ark_ff::PrimeField;
use axum::{Router, routing::{get, post}};
use super::handlers::*;
use axum::extract::Path;
use axum::Json;
use axum::http::StatusCode;
use std::sync::Arc;
use crate::node::BlockchainNode;
use crate::identity_root::{compute_latest_root_from_file, commitments_log_path, read_last_root_anchors};
use chrono::{DateTime, Utc};
use axum::extract::Query;
use serde::Deserialize;

/// Create API routes (blockchain operations, accounts, etc.)
pub fn create_api_routes() -> Router<super::AppState> {
    Router::new()
        // Blockchain routes
        .route("/blocks", get(get_blocks_handler))
        .route("/blocks/:block_number", get(get_block_handler))
        .route("/blocks/latest", get(get_latest_block_handler))
        .route("/blocks/mine", post(mine_block_handler))
        
        // Transaction routes  
        .route("/transactions", post(submit_transaction_handler))

    // Proposal (citizen law proposal) routes
    .route("/proposals", post(create_proposal_handler))
    .route("/proposals", get(list_proposals_handler))
    .route("/proposals/:id", get(get_proposal_handler))
    .route("/proposals/:id/support", post(support_proposal_handler))
        
        // Account routes
        .route("/accounts", post(create_account_handler))
        
        // Identity routes (migrated from api-gateway)
        .route("/identity/:account_id/status", get(identity_status_simple_handler))
    // Identity commitments: public read by hash
    .route("/identity/commitments/:hash", get(get_identity_commitment_by_hash))
    .route("/identity/root", get(get_identity_root_handler))
    .route("/identity/roots", get(get_identity_roots_handler))
    .route("/identity/proof/:commitment_hex", get(get_identity_proof_handler))
    .route("/identity/verify_zkp", post(verify_zkp_handler))
    .route("/identity/commit", post(post_identity_commit))
        
        // Crypto routes (migrated from api-gateway)
        .route("/crypto/verify", post(crypto_verify_handler))
        
        // Vote routes (migrated from api-gateway)
        .route("/vote/:account_id/eligibility", get(vote_eligibility_simple_handler))
        
        // Peer-to-peer routes
    .route("/peers", get(get_peers_handler))
    .route("/peers", post(add_peer_handler))
    .route("/peers/extended", get(get_peers_extended_handler))
    .route("/peers/block", post(receive_block_handler))
    .route("/peers/transaction", post(receive_tx_handler))
    .route("/peers/discover", post(peers_discovery_handler))
    .route("/peers/heartbeat", post(peers_heartbeat_handler))
}

/// Create RPC routes (direct client access)
pub fn create_rpc_routes() -> Router<super::AppState> {
    Router::new()
        .route("/broadcast_transaction", post(broadcast_transaction_handler))
}

// Minimal handler colocated for now (keeps diff small). In larger projects, move to handlers.
async fn get_identity_commitment_by_hash(
    axum::extract::State(node): axum::extract::State<Arc<BlockchainNode>>,
    Path(hash): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Query storage; returns 404 if not found.
    if let Some((did, issuer_did, issued_at, expires_at, status)) = node.storage.get_identity_commitment_by_hash(&hash)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?
    {
        let body = serde_json::json!({
            "did": did,
            "issuer_did": issuer_did,
            "issued_at": issued_at,
            "expires_at": expires_at,
            "status": status,
        });
        Ok(Json(body))
    } else {
        Err((StatusCode::NOT_FOUND, "commitment not found".into()))
    }
}

// POST /identity/commit: verify a presented VC and register its commitment hash
#[derive(serde::Deserialize)]
struct CommitRequest { credential: serde_json::Value }

#[derive(serde::Serialize)]
struct CommitResponse {
    commitment_hash: String,
    did: String,
    issuer_did: String,
    id: i64,
    existed: bool,
}

async fn post_identity_commit(
    axum::extract::State(node): axum::extract::State<Arc<BlockchainNode>>,
    Json(req): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, (StatusCode, String)> {
    // 1) Extract proof fields
    let proof = req.credential.get("proof")
        .and_then(|p| p.as_object())
        .ok_or((StatusCode::BAD_REQUEST, "credential.proof manquant".to_string()))?;
    let verification_method = proof.get("verificationMethod")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "proof.verificationMethod manquant".into()))?;
    let proof_value = proof.get("proofValue")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "proof.proofValue manquant".into()))?;
    if !proof_value.starts_with('z') { return Err((StatusCode::BAD_REQUEST, "proofValue doit commencer par 'z' base58btc".into())); }
    let sig_b58 = &proof_value[1..];
    let sig_bytes = bs58::decode(sig_b58).into_vec().map_err(|e| (StatusCode::BAD_REQUEST, format!("decode signature: {e}")))?;
    if sig_bytes.len()!=64 { return Err((StatusCode::BAD_REQUEST, "signature taille !=64".into())); }

    // 2) Reconstruct unsigned and canonicalize
    let mut unsigned_value = req.credential.clone();
    if let Some(obj) = unsigned_value.as_object_mut() { obj.remove("proof"); }
    let raw_unsigned = serde_json::to_string(&unsigned_value).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize unsigned: {e}")))?;
    let canonical: String = {
        #[cfg(feature = "identity")]
        {
            common::canonical_json_str(&raw_unsigned).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("canonical: {e}")))?
        }
        #[cfg(not(feature = "identity"))]
        {
            let _ = raw_unsigned;
            return Err((StatusCode::NOT_IMPLEMENTED, "identity feature désactivée pour le nœud".into()));
        }
    };

    // 3) Optional digest check
    if let Some(digest_claim) = proof.get("digest").and_then(|d| d.as_str()) {
        use sha2::Digest;
        let recomputed = sha2::Sha256::digest(canonical.as_bytes());
        let recomputed_hex = hex::encode(recomputed);
        if recomputed_hex != digest_claim {
            return Err((StatusCode::BAD_REQUEST, "digest mismatch".into()));
        }
    }

    // 4) Extract parties
    let issuer_did = req.credential.get("issuer").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // Enforce allowed issuers
    let allowed = &node.config.allowed_issuers_dids;
    if !allowed.is_empty() && !allowed.iter().any(|d| d == &issuer_did) {
        return Err((StatusCode::FORBIDDEN, "issuer not allowed".into()));
    }
    let expected_vm = format!("{}#controller", issuer_did);
    if verification_method != expected_vm {
        return Err((StatusCode::BAD_REQUEST, "verificationMethod mismatch".into()));
    }
    let subject_did = req.credential.get("credentialSubject")
        .and_then(|cs| cs.get("id"))
        .and_then(|id| id.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "credentialSubject.id manquant".into()))?
        .to_string();

    // 5) Verify signature using issuer did:key
    if !issuer_did.starts_with("did:key:z") { return Err((StatusCode::BAD_REQUEST, "issuer DID non supporté".into())); }
    let mb = &issuer_did[8..];
    let decoded = bs58::decode(mb.strip_prefix('z').unwrap_or(mb)).into_vec().map_err(|e| (StatusCode::BAD_REQUEST, format!("decode did:key: {e}")))?;
    if decoded.len()!=34 || decoded[0]!=0xED { return Err((StatusCode::BAD_REQUEST, "multicodec ed25519 invalide".into())); }
    let mut pk_bytes = [0u8;32]; pk_bytes.copy_from_slice(&decoded[2..]);
    let pubkey = crypto_lib::PublicKey::from_bytes(&pk_bytes).map_err(|e| (StatusCode::BAD_REQUEST, format!("pubkey: {e}")))?;
    let signature = crypto_lib::Signature::from_bytes(&sig_bytes).map_err(|e| (StatusCode::BAD_REQUEST, format!("signature parse: {e}")))?;
    if let Err(e) = pubkey.verify(canonical.as_bytes(), &signature) {
        return Err((StatusCode::BAD_REQUEST, format!("signature invalide: {e}")));
    }

    // 5.5) Validate dates: issuanceDate present, expirationDate optional but if present must be >= now
    let issued_at_str = req.credential.get("issuanceDate").and_then(|v| v.as_str()).ok_or((StatusCode::BAD_REQUEST, "issuanceDate manquante".into()))?;
    let issued_at_dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(issued_at_str)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("issuanceDate invalide: {e}")))?;
    let now_utc = Utc::now();
    if issued_at_dt.with_timezone(&Utc) > now_utc + chrono::Duration::minutes(5) { // tolérance légère
        return Err((StatusCode::BAD_REQUEST, "credential not yet valid (issuanceDate in future)".into()));
    }
    if let Some(exp_str) = req.credential.get("expirationDate").and_then(|v| v.as_str()) {
        let exp_dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(exp_str)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("expirationDate invalide: {e}")))?;
        if exp_dt.with_timezone(&Utc) < now_utc {
            return Err((StatusCode::BAD_REQUEST, "credential expired".into()));
        }
        if exp_dt <= issued_at_dt {
            return Err((StatusCode::BAD_REQUEST, "expirationDate doit être > issuanceDate".into()));
        }
    }

    // 6) Compute commitment hash and extract dates
    let commitment_hash: String = {
        #[cfg(feature = "identity")]
        {
            let wrapper = common::CitizenCredentialWrapper { raw_credential_json: canonical.clone(), canonical_hash_hex: None };
            let digest = common::compute_credential_hash(&wrapper).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("hash: {e}")))?;
            common::hash_hex(&digest)
        }
        #[cfg(not(feature = "identity"))]
        {
            // unreachable: early return happened above
            String::new()
        }
    };
    let issued_at = issued_at_str.to_string();
    let expires_at = req.credential.get("expirationDate").and_then(|v| v.as_str()).map(|s| s.to_string());

    // 7) Derive subject public key from its did:key (store as public_key)
    let subject_pub_hex = if subject_did.starts_with("did:key:z") {
        let smb = &subject_did[8..];
        let sdec = bs58::decode(smb.strip_prefix('z').unwrap_or(smb)).into_vec().map_err(|e| (StatusCode::BAD_REQUEST, format!("decode subject did:key: {e}")))?;
        if sdec.len()!=34 || sdec[0]!=0xED { return Err((StatusCode::BAD_REQUEST, "subject multicodec ed25519 invalide".into())); }
        hex::encode(&sdec[2..])
    } else { String::new() };

    // 8) Insert into DB (idempotent)
    let (id, existed) = node.storage.insert_identity_commitment(
        &subject_pub_hex,
        &subject_did,
        &commitment_hash,
        &issuer_did,
        &issued_at,
        expires_at.as_deref(),
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db insert: {e}")))?;

    // If this is a new commitment, add it to in-memory set and append to log for Phase 3
    if !existed {
        node.add_active_commitment(&commitment_hash).await;
    }

    Ok(Json(CommitResponse { commitment_hash, did: subject_did, issuer_did, id, existed }))
}

// GET /identity/root: return latest Merkle root and leaf count from commitments.log
async fn get_identity_root_handler(
    axum::extract::State(node): axum::extract::State<Arc<BlockchainNode>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let path = commitments_log_path(&node.config.data_directory);
    let (root, leaves) = compute_latest_root_from_file(&path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("compute root: {e}")))?;
    let root_hex = hex::encode(root);
    Ok(Json(serde_json::json!({
        "root": root_hex,
        "leaves": leaves,
        "path": path.to_string_lossy(),
    })))
}

#[derive(Deserialize)]
struct RootsQuery { limit: Option<usize> }

// GET /identity/roots?limit=N: return latest N anchored roots from identity_roots.jsonl
async fn get_identity_roots_handler(
    axum::extract::State(node): axum::extract::State<Arc<BlockchainNode>>,
    Query(q): Query<RootsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(10).min(1000);
    let entries = read_last_root_anchors(&node.config.data_directory, limit)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("read anchors: {e}")))?;
    Ok(Json(serde_json::json!({
        "count": entries.len(),
        "entries": entries,
        "limit": limit,
    })))
}

// GET /identity/proof/{commitment_hex}: return Merkle inclusion proof for the given commitment
async fn get_identity_proof_handler(
    axum::extract::State(node): axum::extract::State<Arc<BlockchainNode>>,
    Path(commitment_hex): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Decode the commitment hex
    let commitment_bytes = hex::decode(&commitment_hex)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid hex: {e}")))?;
    if commitment_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "commitment must be 32 bytes".into()));
    }
    let mut commitment = [0u8; 32];
    commitment.copy_from_slice(&commitment_bytes);

    // Read all commitments from file
    let path = commitments_log_path(&node.config.data_directory);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err((StatusCode::NOT_FOUND, "no commitments found".into()));
        }
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("read commitments: {e}"))),
    };

    let mut leaves: Vec<common::identity::zkp_prelude::Commitment> = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() { continue; }
        let bytes = hex::decode(t)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("invalid hex at line {}: {e}", idx + 1)))?;
        if bytes.len() != 32 {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("invalid commitment length at line {}", idx + 1)));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        leaves.push(arr);
    }

    // Find the index of the target commitment
    let target_index = leaves.iter().position(|&c| c == commitment)
        .ok_or((StatusCode::NOT_FOUND, "commitment not found in current set".into()))?;

    // Generate the Merkle proof
    let proof = common::identity::zkp_prelude::merkle_proof_for(&leaves, target_index)
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "failed to generate proof".into()))?;

    // Convert to the format needed for ZKP: Vec<(sibling_hex, is_left)>
    let mut path = Vec::new();
    let mut idx = proof.leaf_index;
    for sib in &proof.siblings {
        let is_left = idx % 2 == 0; // if even index, current is left, sibling is right
        path.push((hex::encode(sib), is_left));
        idx /= 2;
    }

    // Also return the root for verification
    let mut acc = common::identity::zkp_prelude::MerkleAccumulator::new();
    for l in &leaves {
        common::identity::zkp_prelude::IdentityAccumulator::append(&mut acc, *l)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("append leaf: {e}")))?;
    }
    let root = common::identity::zkp_prelude::IdentityAccumulator::root(&acc);

    Ok(Json(serde_json::json!({
        "commitment": commitment_hex,
        "leaf_index": proof.leaf_index,
        "root": hex::encode(root),
        "path": path, // Vec<(sibling_hex, is_left)>
    })))
}

// POST /identity/verify_zkp: verify a Groth16 proof envelope produced by the wallet-cli ZkpProve
#[derive(serde::Deserialize)]
struct ZkpVerifyRequest {
    /// either a path to a binary proof file (proof_file) or a hex/base64 string in `proof`
    proof_file: Option<String>,
    proof: Option<String>,
    /// vk version to use
    vk_version: u32,
    /// public inputs as array of 32-byte hex strings in the same order the circuit expects
    public_inputs: Vec<String>,
}

#[derive(serde::Serialize)]
struct ZkpVerifyResponse {
    ok: bool,
    message: String,
}

async fn verify_zkp_handler(
    axum::extract::State(node): axum::extract::State<Arc<BlockchainNode>>,
    Json(req): Json<ZkpVerifyRequest>,
) -> Result<Json<ZkpVerifyResponse>, (StatusCode, String)> {
    #[cfg(not(feature = "zkp_groth16"))]
    {
        return Err((StatusCode::NOT_IMPLEMENTED, "zkp_groth16 feature not enabled on node".into()));
    }

    #[cfg(feature = "zkp_groth16")]
    {
        use crate::zkp_verifier::{verify_groth16_bn254, fr_from_be_bytes};
        use ark_bn254::Fr;

        // Load proof bytes: prefer proof_file if provided
        let proof_bytes: Vec<u8> = if let Some(pf) = req.proof_file.as_ref() {
            // If pf looks like a path and file exists, read it
            if std::path::Path::new(pf).exists() {
                std::fs::read(pf).map_err(|e| (StatusCode::BAD_REQUEST, format!("read proof_file: {e}")))?
            } else {
                return Err((StatusCode::BAD_REQUEST, "proof_file path does not exist".into()));
            }
        } else if let Some(proof_str) = req.proof.as_ref() {
            // Try hex then base64
            if let Ok(b) = hex::decode(proof_str) { b }
            else if let Ok(b) = base64::decode(proof_str) { b }
            else { return Err((StatusCode::BAD_REQUEST, "proof must be hex or base64".into())); }
        } else {
            return Err((StatusCode::BAD_REQUEST, "missing proof_file or proof".into()));
        };

        use std::fs::OpenOptions;
        use std::io::Write;
        use ark_ff::BigInteger;
        let mut log_file = OpenOptions::new().create(true).append(true).open("errors.log").unwrap();
        writeln!(log_file, "[verify_zkp_handler] Received proof_bytes (len={}): {}", proof_bytes.len(), hex::encode(&proof_bytes)).ok();
        writeln!(log_file, "[verify_zkp_handler] Received public_inputs (count={}):", req.public_inputs.len()).ok();
        for (i, h) in req.public_inputs.iter().enumerate() {
            writeln!(log_file, "  [{}] {}", i, h).ok();
        }
        let mut pub_inputs_fr: Vec<Fr> = Vec::with_capacity(req.public_inputs.len());
        for (i, h) in req.public_inputs.iter().enumerate() {
            let b = hex::decode(h).map_err(|e| (StatusCode::BAD_REQUEST, format!("public_inputs[{}] hex decode: {e}", i)))?;
            if b.len() != 32 {
                return Err((StatusCode::BAD_REQUEST, format!("public_inputs[{}] length != 32 bytes", i)));
            }
            let fr = fr_from_be_bytes(&b).ok_or((StatusCode::BAD_REQUEST, format!("public_inputs[{}] convert to Fr failed", i)))?;
            pub_inputs_fr.push(fr);
        }
        writeln!(log_file, "[verify_zkp_handler] Converted public_inputs to Fr:").ok();
        for (i, fr) in pub_inputs_fr.iter().enumerate() {
            writeln!(log_file, "  [{}] {:?}", i, fr.into_bigint()).ok();
        }
        // Log VK hash
        let vk_path = crate::zkp_verifier::vk_path_for_version(&node.config.data_directory, req.vk_version);
        if let Ok(vk_bytes) = std::fs::read(&vk_path) {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&vk_bytes);
            let vk_hash = hasher.finalize();
            writeln!(log_file, "[verify_zkp_handler] VK hash: {}", hex::encode(vk_hash)).ok();
        }

        // Call verifier
    match verify_groth16_bn254(&node.config.data_directory, req.vk_version, &proof_bytes, &pub_inputs_fr, &node.poseidon_params) {
            Ok(true) => Ok(Json(ZkpVerifyResponse { ok: true, message: "verified".into() })),
            Ok(false) => Ok(Json(ZkpVerifyResponse { ok: false, message: "invalid proof".into() })),
            Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("verification error: {e}"))),
        }
    }
}