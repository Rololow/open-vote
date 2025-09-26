use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use axum::Router;
use chrono::Utc;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tokio::task::JoinHandle;

use blockchain_server::api::create_api_router;
use blockchain_server::config::ServerConfig;
use blockchain_server::node::BlockchainNode;
use common::{canonical_json_str, compute_credential_hash, hash_hex};
use crypto_lib::KeyPair;
use sha2::Digest;
use common::{Transaction, TransactionType, Account};

// Helper: ensure MIGRATIONS_DIR points to the workspace migrations folder
fn ensure_migrations_env() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_dir = crate_dir.parent().unwrap_or(&crate_dir);
    let migrations_dir = workspace_dir.join("migrations");
    std::env::set_var("MIGRATIONS_DIR", migrations_dir.to_string_lossy().to_string());
}

// Helper: create minimal tables needed by storage and identity lookups
async fn ensure_minimal_schema(db_url: &str) {
    let opts = SqliteConnectOptions::from_str(db_url).unwrap().create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await.unwrap();

    // blocks table (subset sufficient for load_blockchain query)
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS blocks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_number INTEGER NOT NULL UNIQUE,
            hash TEXT NOT NULL UNIQUE,
            previous_hash TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            block_data TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // identity_commitments table
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS identity_commitments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            public_key TEXT NOT NULL,
            did TEXT NOT NULL,
            commitment_hash TEXT NOT NULL UNIQUE,
            issuer_did TEXT NOT NULL,
            issued_at TEXT NOT NULL,
            expires_at TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(&pool)
    .await
    .unwrap();
}

// Helper: encode did:key for an Ed25519 public key (multicodec 0xED 0x01)
fn did_key_from_pubkey(pk: &crypto_lib::PublicKey) -> String {
    let mut data = Vec::with_capacity(34);
    data.push(0xED);
    data.push(0x01);
    data.extend_from_slice(&pk.to_bytes());
    let mb = bs58::encode(data).into_string();
    format!("did:key:z{}", mb)
}

// Helper: start the API server on an ephemeral port, returning base URL and the join handle
async fn start_test_server(allowed_issuers: Vec<String>, db_suffix: &str) -> (String, JoinHandle<anyhow::Result<()>>) {
    ensure_migrations_env();
    let mut cfg = ServerConfig::default();
    cfg.bind_address = "127.0.0.1".to_string();
    cfg.api_port = 0; // we'll bind manually to port 0
    cfg.p2p_port = 0;
    cfg.database_url = format!("sqlite://./data/test_api_identity_{}.db", db_suffix);
    cfg.allowed_issuers_dids = allowed_issuers;

    ensure_minimal_schema(&cfg.database_url).await;

    let node = BlockchainNode::new(cfg.clone()).await.expect("node");
    let app: Router = create_api_router(std::sync::Arc::new(node));

    // Bind to 127.0.0.1:0 to get an ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let local: SocketAddr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}:{}", local.ip(), local.port());

    let handle = tokio::spawn(async move { Ok(axum::serve(listener, app).await?) });
    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    (base_url, handle)
}

// Build a minimal citizen VC and sign it with issuer key
fn build_signed_vc(issuer_kp: &KeyPair, issuer_did: &str, subject_did: &str) -> serde_json::Value {
    let issuance = Utc::now();
    let expiration = issuance + chrono::Duration::days(30);

    // Unsigned credential
    let mut cred = serde_json::json!({
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "CitizenCredential"],
        "issuer": issuer_did,
        "issuanceDate": issuance.to_rfc3339(),
        "expirationDate": expiration.to_rfc3339(),
        "credentialSubject": {
            "id": subject_did,
            "nationalId": "FR-123456789",
            "country": "FR",
        }
    });

    // Canonicalize without proof
    let mut unsigned = cred.clone();
    if let Some(obj) = unsigned.as_object_mut() { obj.remove("proof"); }
    let raw = serde_json::to_string(&unsigned).unwrap();
    let canonical = canonical_json_str(&raw).unwrap();

    // Optional digest
    use sha2::Digest;
    let digest_hex = hex::encode(sha2::Sha256::digest(canonical.as_bytes()));

    // Sign canonical with issuer
    let sig = issuer_kp.sign(canonical.as_bytes());
    let proof_value = format!("z{}", bs58::encode(sig.to_bytes()).into_string());
    let verification_method = format!("{}#controller", issuer_did);

    // Attach proof
    let proof = serde_json::json!({
        "type": "Ed25519Signature2020",
        "verificationMethod": verification_method,
        "proofValue": proof_value,
        "digest": digest_hex,
    });
    if let Some(obj) = cred.as_object_mut() { obj.insert("proof".into(), proof); }

    cred
}

#[tokio::test]
async fn post_identity_commit_success() {
    // Prepare issuer/subject
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    // Start server allowing the issuer
    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build signed VC
    let vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);

    // Compute expected commitment hash
    let mut unsigned = vc.clone();
    if let Some(obj) = unsigned.as_object_mut() { obj.remove("proof"); }
    let raw = serde_json::to_string(&unsigned).unwrap();
    let canonical = canonical_json_str(&raw).unwrap();
    let wrapper = common::CitizenCredentialWrapper { raw_credential_json: canonical.clone(), canonical_hash_hex: None };
    let digest = compute_credential_hash(&wrapper).unwrap();
    let expected_commitment = hash_hex(&digest);

    let client = reqwest::Client::new();
    let url = format!("{}/api/identity/commit", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await
        .expect("http");
    assert!(resp.status().is_success(), "status: {}", resp.status());
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["commitment_hash"].as_str(), Some(expected_commitment.as_str()));
    assert_eq!(body["issuer_did"].as_str(), Some(issuer_did.as_str()));
    assert_eq!(body["did"].as_str(), Some(subject_did.as_str()));

    // Verify public GET works
    let get_url = format!("{}/api/identity/commitments/{}", base, expected_commitment);
    let get_resp = client.get(&get_url).send().await.expect("get");
    assert!(get_resp.status().is_success(), "get status: {}", get_resp.status());
    let get_body: serde_json::Value = get_resp.json().await.expect("get json");
    assert_eq!(get_body["issuer_did"].as_str(), Some(issuer_did.as_str()));
    assert_eq!(get_body["did"].as_str(), Some(subject_did.as_str()));
}

#[tokio::test]
async fn post_identity_commit_rejects_disallowed_issuer() {
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    // Start server with a different allowed issuer
    let (base, _handle) = start_test_server(vec!["did:key:zOtherIssuer".to_string()], &uuid::Uuid::new_v4().to_string()).await;

    let vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);
    let client = reqwest::Client::new();
    let url = format!("{}/api/identity/commit", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await
        .expect("http");
    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn post_identity_commit_rejects_invalid_signature() {
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build VC then tamper proofValue to break signature
    let mut vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);
    if let Some(p) = vc.get_mut("proof") { if let Some(obj) = p.as_object_mut() { if let Some(v) = obj.get_mut("proofValue") { *v = serde_json::Value::String(format!("{}A", v.as_str().unwrap_or(""))); } } }

    let client = reqwest::Client::new();
    let url = format!("{}/api/identity/commit", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await
        .expect("http");
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_identity_commit_rejects_digest_mismatch() {
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build VC then corrupt digest to mismatch canonical hash
    let mut vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);
    if let Some(p) = vc.get_mut("proof") { if let Some(obj) = p.as_object_mut() { obj.insert("digest".into(), serde_json::json!("deadbeef")); } }

    let client = reqwest::Client::new();
    let url = format!("{}/api/identity/commit", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await
        .expect("http");
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_identity_commit_rejects_expired_vc() {
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build VC then set expirationDate in the past
    let mut vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);
    if let Some(obj) = vc.as_object_mut() {
        obj.insert("expirationDate".into(), serde_json::json!((Utc::now() - chrono::Duration::days(1)).to_rfc3339()));
    }
    // Recompute signature since canonical changed (we want expired but otherwise valid signature)
    let mut unsigned = vc.clone();
    if let Some(o) = unsigned.as_object_mut() { o.remove("proof"); }
    let raw = serde_json::to_string(&unsigned).unwrap();
    let canonical = canonical_json_str(&raw).unwrap();
    let sig = issuer_kp.sign(canonical.as_bytes());
    let proof_value = format!("z{}", bs58::encode(sig.to_bytes()).into_string());
    let verification_method = format!("{}#controller", issuer_did);
    let proof = serde_json::json!({
        "type": "Ed25519Signature2020",
        "verificationMethod": verification_method,
        "proofValue": proof_value,
        "digest": hex::encode(sha2::Sha256::digest(canonical.as_bytes())),
    });
    if let Some(o) = vc.as_object_mut() { o.insert("proof".into(), proof); }

    let client = reqwest::Client::new();
    let url = format!("{}/api/identity/commit", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await
        .expect("http");
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn e2e_commit_then_submit_tx_with_identity_and_mine() {
    // Prepare issuer/subject
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    // Start server allowing the issuer
    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build signed VC and compute expected commitment hash
    let vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);
    let mut unsigned = vc.clone();
    if let Some(obj) = unsigned.as_object_mut() { obj.remove("proof"); }
    let raw = serde_json::to_string(&unsigned).unwrap();
    let canonical = canonical_json_str(&raw).unwrap();
    let wrapper = common::CitizenCredentialWrapper { raw_credential_json: canonical.clone(), canonical_hash_hex: None };
    let digest = compute_credential_hash(&wrapper).unwrap();
    let commitment = hash_hex(&digest);

    // POST /api/identity/commit
    let client = reqwest::Client::new();
    let commit_url = format!("{}/api/identity/commit", base);
    let resp = client.post(&commit_url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await.expect("http");
    assert!(resp.status().is_success(), "commit status: {}", resp.status());

    // Confirm public GET
    let get_url = format!("{}/api/identity/commitments/{}", base, commitment);
    let get_resp = client.get(&get_url).send().await.expect("get");
    assert!(get_resp.status().is_success());

    // Construct a signed transaction with identity_ref
    let tx_kp = KeyPair::generate();
    let account = Account::new(tx_kp.public_key().clone());
    let ttype = TransactionType::CreateAccount(account);
    let mut tx = Transaction::new(ttype, tx_kp.public_key().clone(), tx_kp.sign(b"temp"), 1, 0);
    let msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
    tx.signature = tx_kp.sign(msg.as_bytes());
    tx.identity_ref = Some(commitment.clone());

    // Broadcast via /rpc/broadcast_transaction
    let rpc_url = format!("{}/rpc/broadcast_transaction", base);
    let bresp = client.post(&rpc_url)
        .json(&serde_json::json!({"transaction": tx}))
        .send().await.expect("broadcast");
    assert!(bresp.status().is_success(), "broadcast status: {}", bresp.status());

    // Mine a block
    let mine_url = format!("{}/api/blocks/mine", base);
    let mresp = client.post(&mine_url).send().await.expect("mine");
    assert!(mresp.status().is_success(), "mine status: {}", mresp.status());

    // Fetch latest block and verify inclusion of our identity_ref
    let latest_url = format!("{}/api/blocks/latest", base);
    let lresp = client.get(&latest_url).send().await.expect("latest");
    assert!(lresp.status().is_success());
    let block_json: serde_json::Value = lresp.json().await.expect("json");
    let txs = block_json.get("transactions").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert!(!txs.is_empty(), "expected at least one transaction in mined block");
    let found = txs.iter().any(|t| t.get("identity_ref").and_then(|v| v.as_str()) == Some(commitment.as_str()));
    assert!(found, "mined block should contain a tx with our identity_ref");
}
