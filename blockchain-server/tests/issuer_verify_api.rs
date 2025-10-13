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
use common::canonical_json_str;
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
    cfg.database_url = format!("sqlite://./data/test_api_issuer_verify_{}.db", db_suffix);
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
async fn issuer_verify_success() {
    // Prepare issuer/subject
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    // Start server allowing the issuer
    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build signed VC
    let vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);

    // POST /issuer/verify
    let client = reqwest::Client::new();
    let url = format!("{}/issuer/verify", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await.expect("http");
    assert!(resp.status().is_success(), "status: {}", resp.status());
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["valid"].as_bool(), Some(true));
    assert_eq!(body["issuer_did"].as_str(), Some(issuer_did.as_str()));
    assert_eq!(body["subject_did"].as_str(), Some(subject_did.as_str()));
    assert!(body.get("commitment_hash").and_then(|v| v.as_str()).is_some());
}

#[tokio::test]
async fn issuer_verify_signature_alt_fails() {
    // Prepare issuer/subject
    let issuer_kp = KeyPair::generate();
    let subject_kp = KeyPair::generate();
    let issuer_did = did_key_from_pubkey(issuer_kp.public_key());
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    // Start server allowing the issuer
    let (base, _handle) = start_test_server(vec![issuer_did.clone()], &uuid::Uuid::new_v4().to_string()).await;

    // Build VC and then alter the signature bytes (flip one bit) but keep length 64
    let mut vc = build_signed_vc(&issuer_kp, &issuer_did, &subject_did);
    if let Some(p) = vc.get_mut("proof") {
        if let Some(obj) = p.as_object_mut() {
            if let Some(v) = obj.get_mut("proofValue") {
                let orig = v.as_str().unwrap_or("");
                assert!(orig.starts_with('z'));
                let b58 = &orig[1..];
                let mut bytes = bs58::decode(b58).into_vec().expect("decode orig sig");
                assert_eq!(bytes.len(), 64);
                // flip a bit
                bytes[0] ^= 0x01;
                let tampered_b58 = bs58::encode(bytes).into_string();
                *v = serde_json::Value::String(format!("z{}", tampered_b58));
            }
        }
    }

    // POST /issuer/verify
    let client = reqwest::Client::new();
    let url = format!("{}/issuer/verify", base);
    let resp = client.post(&url)
        .json(&serde_json::json!({"credential": vc}))
        .send().await.expect("http");
    assert!(resp.status().is_success(), "status: {}", resp.status());
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["valid"].as_bool(), Some(false));
    let reason = body["reason"].as_str().unwrap_or("");
    assert!(reason.contains("signature invalide"), "unexpected reason: {}", reason);
}

#[tokio::test]
async fn e2e_full_issue_commit_submit_via_issuer_endpoint() {
    // Use a temp issuer key file to avoid cross-test interference
    let tmp_key = std::env::temp_dir().join(format!("issuer_key_{}.json", uuid::Uuid::new_v4()));
    std::env::set_var("ISSUER_KEY_PATH", tmp_key.to_string_lossy().to_string());

    // Prepare subject DID
    let subject_kp = KeyPair::generate();
    let subject_did = did_key_from_pubkey(subject_kp.public_key());

    // Start server with no allowlist (accept any issuer for this test)
    let (base, _handle) = start_test_server(vec![], &uuid::Uuid::new_v4().to_string()).await;

    let client = reqwest::Client::new();

    // 1) Issue credential via /issuer/credential
    let issue_url = format!("{}/issuer/credential", base);
    let issue_resp = client
        .post(&issue_url)
        .json(&serde_json::json!({"subject_did": subject_did}))
        .send()
        .await
        .expect("issue http");
    assert!(issue_resp.status().is_success(), "issue status: {}", issue_resp.status());
    let issue_body: serde_json::Value = issue_resp.json().await.expect("issue json");
    let vc = issue_body
        .get("credential")
        .cloned()
        .expect("credential field");

    // 2) Verify via /issuer/verify
    let verify_url = format!("{}/issuer/verify", base);
    let vresp = client
        .post(&verify_url)
        .json(&serde_json::json!({"credential": vc.clone()}))
        .send()
        .await
        .expect("verify http");
    assert!(vresp.status().is_success());
    let vjson: serde_json::Value = vresp.json().await.expect("verify json");
    assert_eq!(vjson["valid"].as_bool(), Some(true));

    // 3) Commit via /api/identity/commit
    let commit_url = format!("{}/api/identity/commit", base);
    let cresp = client
        .post(&commit_url)
        .json(&serde_json::json!({"credential": vc}))
        .send()
        .await
        .expect("commit http");
    assert!(cresp.status().is_success(), "commit status: {}", cresp.status());
    let cjson: serde_json::Value = cresp.json().await.expect("commit json");
    let commitment = cjson["commitment_hash"].as_str().expect("commitment_hash").to_string();

    // 4) Submit a tx referencing the commitment and mine it
    let tx_kp = KeyPair::generate();
    let account = Account::new(tx_kp.public_key().clone());
    let ttype = TransactionType::CreateAccount(account);
    let mut tx = Transaction::new(ttype, tx_kp.public_key().clone(), tx_kp.sign(b"temp"), 1, 0);
    let sign_msg = format!(
        "TRANSACTION:{}:{}:{}",
        tx.id,
        tx.timestamp,
        tx.data_hash.to_hex()
    );
    tx.signature = tx_kp.sign(sign_msg.as_bytes());
    tx.identity_ref = Some(commitment.clone());

    let rpc_url = format!("{}/rpc/broadcast_transaction", base);
    let bresp = client
        .post(&rpc_url)
        .json(&serde_json::json!({"transaction": tx}))
        .send()
        .await
        .expect("broadcast http");
    assert!(bresp.status().is_success(), "broadcast status: {}", bresp.status());

    let mine_url = format!("{}/api/blocks/mine", base);
    let mresp = client.post(&mine_url).send().await.expect("mine http");
    assert!(mresp.status().is_success(), "mine status: {}", mresp.status());

    let latest_url = format!("{}/api/blocks/latest", base);
    let lresp = client.get(&latest_url).send().await.expect("latest http");
    assert!(lresp.status().is_success());
    let block_json: serde_json::Value = lresp.json().await.expect("latest json");
    let txs = block_json
        .get("transactions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!txs.is_empty(), "expected at least one transaction");
    let found = txs
        .iter()
        .any(|t| t.get("identity_ref").and_then(|v| v.as_str()) == Some(commitment.as_str()));
    assert!(found, "mined block should contain a tx with our identity_ref");
}