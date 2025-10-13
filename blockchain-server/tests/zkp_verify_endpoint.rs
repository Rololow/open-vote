#![cfg(feature = "zkp_groth16")]

use std::fs;
use std::io::Write;

use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, VerifyingKey};
use ark_snark::SNARK;
use ark_snark::CircuitSpecificSetupSNARK;
use ark_ff::{PrimeField, BigInteger};
use rand::thread_rng;

use axum::Router;
use tokio::task::JoinHandle;
use tempfile::tempdir;

use blockchain_server::api::create_api_router;
use blockchain_server::config::ServerConfig;
use blockchain_server::node::BlockchainNode;
use blockchain_server::zkp_verifier::vk_path_for_version;
// Import the schema helper from node tests
use blockchain_server::ensure_minimal_schema;

// Reuse small AddCircuit idea from existing tests: public inputs [c], witnesses a,b with c = a + b
#[derive(Clone, Default)]
struct AddCircuit<F: ark_ff::Field> {
    pub a: Option<F>,
    pub b: Option<F>,
    pub c: Option<F>,
}

impl<F: ark_ff::Field> ark_relations::r1cs::ConstraintSynthesizer<F> for AddCircuit<F> {
    fn generate_constraints(self, cs: ark_relations::r1cs::ConstraintSystemRef<F>) -> Result<(), ark_relations::r1cs::SynthesisError> {
        let a = self.a.ok_or(ark_relations::r1cs::SynthesisError::AssignmentMissing)?;
        let b = self.b.ok_or(ark_relations::r1cs::SynthesisError::AssignmentMissing)?;
        let c = self.c.ok_or(ark_relations::r1cs::SynthesisError::AssignmentMissing)?;
        let a_var = cs.new_witness_variable(|| Ok(a))?;
        let b_var = cs.new_witness_variable(|| Ok(b))?;
        let c_var = cs.new_input_variable(|| Ok(c))?;
        // enforce a + b - c = 0
        let mut lc = ark_relations::r1cs::LinearCombination::<F>::zero();
        lc = lc + (F::one(), a_var);
        lc = lc + (F::one(), b_var);
        lc = lc + (-F::one(), c_var);
        cs.enforce_constraint(lc, ark_relations::r1cs::LinearCombination::zero(), ark_relations::r1cs::LinearCombination::zero())?;
        Ok(())
    }
}

// Start a minimal node and server similarly to other tests
async fn start_server_for_test(data_dir: &str, db_path: &str) -> (String, JoinHandle<anyhow::Result<()>>) {
    let mut cfg = ServerConfig::default();
    cfg.bind_address = "127.0.0.1".to_string();
    cfg.api_port = 0; // ephemeral
    cfg.p2p_port = 0;
    cfg.data_directory = data_dir.to_string();
    cfg.database_url = db_path.to_string();
    let node = BlockchainNode::new(cfg.clone()).await.expect("node");
    let app: Router = create_api_router(std::sync::Arc::new(node));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let local = listener.local_addr().unwrap();
    let base = format!("http://{}:{}", local.ip(), local.port());
    let handle = tokio::spawn(async move { Ok(axum::serve(listener, app).await?) });
    // small sleep to let server start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (base, handle)
}

#[tokio::test]
async fn zkp_verify_endpoint_roundtrip() {
    // Create temp data dir
    let tmp = tempdir().unwrap();
    let data_dir = tmp.path().to_str().unwrap().to_string();

    // Ensure DB schema exists (create blocks table etc)
    let db_path = format!("sqlite://{}/blockchain.db", data_dir);
    ensure_minimal_schema(&db_path).await;

    // Generate a small Groth16 keypair for AddCircuit
    let mut rng = thread_rng();
    let setup_circuit = AddCircuit::<Fr> { a: Some(Fr::from(0u64)), b: Some(Fr::from(0u64)), c: Some(Fr::from(0u64)) };
    let (pk, vk) = Groth16::<Bn254>::setup(setup_circuit, &mut rng).expect("setup");

    // write vk to data_dir/zkp/vk-groth16-v1.bin
    let vk_path = vk_path_for_version(&data_dir, 1);
    fs::create_dir_all(vk_path.parent().unwrap()).unwrap();
    let mut vk_bytes = Vec::new();
    ark_serialize::CanonicalSerialize::serialize_compressed(&vk, &mut vk_bytes).unwrap();
    let mut f = fs::File::create(&vk_path).unwrap();
    f.write_all(&vk_bytes).unwrap();
    drop(f);

    // Create a real proof for a=5, b=7, c=12
    let a = Fr::from(5u64);
    let b = Fr::from(7u64);
    let c = a + b;
    let circuit = AddCircuit { a: Some(a), b: Some(b), c: Some(c) };
    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove");
    let mut proof_bytes = Vec::new();
    ark_serialize::CanonicalSerialize::serialize_compressed(&proof, &mut proof_bytes).unwrap();

    // Start server
    let (base, _handle) = start_server_for_test(&data_dir, &db_path).await;

    // Post to /api/identity/verify_zkp with proof as hex and public_inputs [c]
    let client = reqwest::Client::new();
    let proof_hex = hex::encode(&proof_bytes);
    let c_bytes = {
        let mut b = Vec::new();
        ark_serialize::CanonicalSerialize::serialize_compressed(&c, &mut b).unwrap();
        // Convert Fr to 32-byte BE representation by using into_bigint
        use ark_ff::BigInteger;
        let be = c.into_bigint().to_bytes_be();
        let mut buf = vec![0u8; 32 - be.len()]; buf.extend_from_slice(&be); buf
    };
    let c_hex = hex::encode(c_bytes);

    let url = format!("{}/api/identity/verify_zkp", base);
    let body = serde_json::json!({
        "vk_version": 1u32,
        "public_inputs": [c_hex],
        "proof": proof_hex,
    });
    let resp = client.post(&url).json(&body).send().await.expect("http");
    assert!(resp.status().is_success(), "status {}", resp.status());
    let v: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(v["ok"].as_bool().unwrap_or(false), true, "verify should succeed");
}
