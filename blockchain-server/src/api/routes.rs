use axum::{Router, routing::{get, post}};
use super::handlers::*;
use axum::extract::Path;
use axum::Json;
use axum::http::StatusCode;
use std::sync::Arc;
use crate::node::BlockchainNode;
use chrono::{DateTime, Utc};

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