use axum::{Router, routing::{get, post}, Json, extract::State};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use axum::http::StatusCode;
use crate::node::BlockchainNode;
use crate::issuer::{IssuerConfig, load_or_create_key, build_unsigned_credential, sign_credential_with_artifacts, derive_did_key_ed25519};
#[cfg(feature="identity")] use common::{CitizenCredentialWrapper, compute_credential_hash, hash_hex};
use tracing::info;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

type AppState = Arc<BlockchainNode>;

#[derive(Serialize)]
struct JwkExportStub {
    kty: &'static str,
    crv: &'static str,
    x: String,
    kid: String,
    issuer_did: String,
    note: &'static str,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/jwk", get(get_jwk))
        .route("/credential", post(issue_credential))
        .route("/verify", post(verify_credential))
}

async fn get_jwk(State(_node): State<AppState>) -> Json<JwkExportStub> {
    let cfg = IssuerConfig::from_env();
    let kp = load_or_create_key(&cfg).expect("issuer key load");
    let pub_bytes = kp.public_key.to_bytes();
    let issuer_did = derive_did_key_ed25519(&pub_bytes);
    let x_b64 = URL_SAFE_NO_PAD.encode(pub_bytes);
    // Derive a deterministic kid (key id) from the public key (first 16 hex chars)
    let pub_hex = hex::encode(pub_bytes);
    let kid = format!("ed25519-{}", &pub_hex[..16]);
    // Optionally export public JWK to a file if ISSUER_PUBKEY_EXPORT is set
    if let Ok(path) = std::env::var("ISSUER_PUBKEY_EXPORT") {
        let jwk_json = serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": x_b64,
            "kid": kid,
            "issuer_did": issuer_did,
        });
        if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&jwk_json).unwrap_or_else(|_| "{}".into())) {
            tracing::warn!("Impossible d'exporter le JWK public vers {}: {}", path, e);
        }
    }
    Json(JwkExportStub {
        kty: "OKP",
        crv: "Ed25519",
        x: x_b64,
        kid,
        issuer_did,
        note: "Stub JWK export – structure minimale (Phase 2 WIP)",
    })
}

#[derive(Debug, Deserialize)]
struct IssueCredentialRequest {
    subject_did: String,
}

#[derive(Serialize)]
struct CredentialResponse { credential: serde_json::Value }

async fn issue_credential(
    State(_node): State<AppState>,
    Json(payload): Json<IssueCredentialRequest>
) -> Result<Json<CredentialResponse>, (StatusCode, String)> {
    // Validation minimale DID (placeholder – future résolution DID plus stricte)
    if !payload.subject_did.starts_with("did:key:") {
        return Err((StatusCode::BAD_REQUEST, "subject_did doit commencer par did:key:".into()));
    }
    let mut cfg = IssuerConfig::from_env();
    let kp = load_or_create_key(&cfg).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("load key: {e}")))?;
    let issuer_did = derive_did_key_ed25519(&kp.public_key.to_bytes());
    cfg.did = issuer_did.clone();
    let unsigned = build_unsigned_credential(&cfg, &payload.subject_did);
    let (signed, artifacts) = sign_credential_with_artifacts(&kp, &unsigned)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("sign: {e}")))?;

    // Calcul commitment hash (JSON unsigned sans preuve) via wrapper
    #[cfg(feature="identity")] let commitment_hash = {
        // artifacts.canonical_json already canonicalizes the unsigned credential content
        let wrapper = CitizenCredentialWrapper { raw_credential_json: artifacts.canonical_json.clone(), canonical_hash_hex: None };
        let digest = compute_credential_hash(&wrapper).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("hash: {e}")))?;
        hash_hex(&digest)
    };
    #[cfg(not(feature="identity"))] let commitment_hash = String::from("identity_feature_disabled");

    // Insertion DB idempotente
    let issued_at = &unsigned.issuanceDate;
    let expires_at = &unsigned.expirationDate;
    let storage = &_node.storage; // Arc<Storage>
    let insert_res = storage.insert_identity_commitment(
        &kp.public_key.to_hex(),
        &payload.subject_did,
        &commitment_hash,
        &issuer_did,
        issued_at,
        Some(expires_at)
    ).await;
    let (id, existed) = match insert_res { Ok(v) => v, Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("db insert: {e}"))) };
    let sig_prefix = &artifacts.signature_base58[..std::cmp::min(16, artifacts.signature_base58.len())];
    info!("credential_issued id={} existed={} hash={} digest={} sig_b58_prefix={} issuer_did={}",
          id, existed, commitment_hash, artifacts.digest_sha256_hex, sig_prefix, issuer_did);

    let value = serde_json::to_value(&signed).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize: {e}")))?;
    Ok(Json(CredentialResponse { credential: value }))
}

#[derive(Debug, Deserialize)]
struct VerifyRequest { credential: serde_json::Value }

#[derive(Serialize)]
struct VerifyResponse {
    valid: bool,
    issuer_did: String,
    subject_did: Option<String>,
    commitment_hash: Option<String>,
    reason: Option<String>,
}

async fn verify_credential(
    State(node): State<AppState>,
    Json(req): Json<VerifyRequest>
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    // Basic shape: expect proof + unsigned flattened
    let proof = req.credential.get("proof")
        .and_then(|p| p.as_object())
        .ok_or((StatusCode::BAD_REQUEST, "credential.proof manquant".to_string()))?;

    let verification_method = proof.get("verificationMethod")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "proof.verificationMethod manquant".into()))?;

    // Extract signature (proofValue starts with 'z')
    let proof_value = proof.get("proofValue")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "proof.proofValue manquant".into()))?;
    if !proof_value.starts_with('z') { return Err((StatusCode::BAD_REQUEST, "proofValue doit commencer par 'z' base58btc".into())); }
    let sig_b58 = &proof_value[1..];
    let sig_bytes = bs58::decode(sig_b58).into_vec().map_err(|e| (StatusCode::BAD_REQUEST, format!("decode signature: {e}")))?;
    if sig_bytes.len()!=64 { return Err((StatusCode::BAD_REQUEST, "signature taille !=64".into())); }

    // Reconstruct unsigned portion (remove proof, then canonicalize deterministically like during signing)
    let mut unsigned_value = req.credential.clone();
    if let Some(obj) = unsigned_value.as_object_mut() { obj.remove("proof"); }
    let raw_unsigned = serde_json::to_string(&unsigned_value).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("serialize unsigned: {e}")))?;
    #[cfg(feature="identity")] let canonical = common::canonical_json_str(&raw_unsigned).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("canonical: {e}")))?;
    #[cfg(not(feature="identity"))] let canonical = raw_unsigned;

    // Digest check if present in proof
    if let Some(digest_claim) = proof.get("digest").and_then(|d| d.as_str()) {
        use sha2::Digest; // local import
        let recomputed = sha2::Sha256::digest(canonical.as_bytes());
        let recomputed_hex = hex::encode(recomputed);
        if recomputed_hex != digest_claim {
            return Ok(Json(VerifyResponse { valid: false, issuer_did: String::new(), subject_did: None, commitment_hash: None, reason: Some("digest mismatch".into()) }));
        }
    }

    // Extract issuer DID & subject DID
    let issuer_did = req.credential.get("issuer").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Enforce allowed issuers list if configured
    let allowed = &node.config.allowed_issuers_dids;
    if !allowed.is_empty() && !allowed.iter().any(|d| d == &issuer_did) {
        return Ok(Json(VerifyResponse { valid: false, issuer_did, subject_did: None, commitment_hash: None, reason: Some("issuer not allowed".into()) }));
    }
    // Validate verificationMethod linkage to issuer DID (expected suffix '#controller')
    let expected_vm = format!("{}#controller", issuer_did);
    if verification_method != expected_vm {
        return Ok(Json(VerifyResponse { valid: false, issuer_did, subject_did: None, commitment_hash: None, reason: Some("verificationMethod mismatch".into()) }));
    }
    let subject_did = req.credential.get("credentialSubject")
        .and_then(|cs| cs.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    // Derive public key from issuer_did did:key (Ed25519) naive extraction (strip prefix did:key:z and multicodec header)
    if !issuer_did.starts_with("did:key:z") { return Ok(Json(VerifyResponse { valid: false, issuer_did, subject_did, commitment_hash: None, reason: Some("issuer DID non supporté".into()) })); }
    let mb = &issuer_did[8..]; // after did:key:
    let decoded = bs58::decode(mb.strip_prefix('z').unwrap_or(mb)).into_vec().map_err(|e| (StatusCode::BAD_REQUEST, format!("decode did:key: {e}")))?;
    if decoded.len()!=34 || decoded[0]!=0xED { return Ok(Json(VerifyResponse { valid: false, issuer_did, subject_did, commitment_hash: None, reason: Some("multicodec ed25519 invalide".into()) })); }
    let mut pk_bytes = [0u8;32]; pk_bytes.copy_from_slice(&decoded[2..]);
    let pubkey = crypto_lib::PublicKey::from_bytes(&pk_bytes).map_err(|e| (StatusCode::BAD_REQUEST, format!("pubkey: {e}")))?;
    let signature = crypto_lib::Signature::from_bytes(&sig_bytes).map_err(|e| (StatusCode::BAD_REQUEST, format!("signature parse: {e}")))?;

    // Verify
    if let Err(e) = pubkey.verify(canonical.as_bytes(), &signature) {
        return Ok(Json(VerifyResponse { valid: false, issuer_did, subject_did, commitment_hash: None, reason: Some(format!("signature invalide: {e}")) }));
    }

    // Compute commitment hash (optional) for caller convenience
    #[cfg(feature="identity")] let commitment_hash = {
        let wrapper = CitizenCredentialWrapper { raw_credential_json: canonical.clone(), canonical_hash_hex: None }; // proof already removed
        match compute_credential_hash(&wrapper) { Ok(d) => Some(hash_hex(&d)), Err(_) => None }
    };
    #[cfg(not(feature="identity"))] let commitment_hash = None;

    Ok(Json(VerifyResponse { valid: true, issuer_did, subject_did, commitment_hash, reason: None }))
}
