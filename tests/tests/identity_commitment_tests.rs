use reqwest::Client;
use tokio::time::{sleep, Duration};
use serde_json::Value;

#[cfg(feature="identity")]
use common::{CitizenCredentialWrapper, compute_credential_hash, hash_hex};

// NOTE: This test assumes the blockchain-server with identity feature is running locally on port 3000.
// It exercises the issuer POST /issuer/credential endpoint for idempotent commitment insertion.
#[tokio::test]
async fn test_commitment_insertion_idempotent() {
    // Precondition: server running. If not, skip.
    let client = Client::new();
    let url = "http://localhost:3000/issuer/credential";

    let subject_did = "did:key:zTESTSUBJECT_INTEGRATION";
    let body = serde_json::json!({"subject_did": subject_did});

    // First issuance (soft skip if server not running)
    let resp1 = match client.post(url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Skipping test_commitment_insertion_idempotent: server unreachable: {e}");
            return; // soft skip
        }
    };
    assert!(resp1.status().is_success(), "first issuance failed: {}", resp1.status());
    let v1: Value = resp1.json().await.expect("json 1");
    let cred1 = v1.get("credential").cloned().expect("credential field");

    // Compute commitment hash locally (remove proof before canonical hash) to compare stability
    #[cfg(feature="identity")]
    let local_hash1 = {
        let mut unsigned1 = cred1.clone();
        if let Value::Object(ref mut m) = unsigned1 { m.remove("proof"); }
        let raw1 = serde_json::to_string(&unsigned1).unwrap();
        let wrapper1 = CitizenCredentialWrapper { raw_credential_json: raw1, canonical_hash_hex: None };
        hash_hex(&compute_credential_hash(&wrapper1).expect("hash1"))
    };

    // Second issuance with same subject DID -> should reuse same commitment in DB, returning (hash identical)
    sleep(Duration::from_millis(50)).await; // slight delay
    let resp2 = client.post(url).json(&body).send().await
        .expect("issuer server not reachable second");
    assert!(resp2.status().is_success(), "second issuance failed: {}", resp2.status());
    let v2: Value = resp2.json().await.expect("json 2");
    let cred2 = v2.get("credential").cloned().expect("credential field 2");

    #[cfg(feature="identity")]
    {
        let mut unsigned2 = cred2.clone();
        if let Value::Object(ref mut m) = unsigned2 { m.remove("proof"); }
        let raw2 = serde_json::to_string(&unsigned2).unwrap();
        let wrapper2 = CitizenCredentialWrapper { raw_credential_json: raw2, canonical_hash_hex: None };
        let local_hash2 = hash_hex(&compute_credential_hash(&wrapper2).expect("hash2"));
        assert_eq!(local_hash1, local_hash2, "Commitment hash should be identical for repeated issuance");
    }

    // Basic structural checks
    assert_eq!(cred1["credentialSubject"]["id"], subject_did);
    assert_eq!(cred2["credentialSubject"]["id"], subject_did);
}
