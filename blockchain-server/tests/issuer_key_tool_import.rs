use std::process::Command;

use blockchain_server::issuer::{IssuerConfig, load_or_create_key, derive_did_key_ed25519};

fn tmp_key_path() -> std::path::PathBuf {
    // Make the temporary path more robustly unique by including process id and thread id
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let pid = std::process::id();
    let tid = format!("{:?}", std::thread::current().id());
    std::env::temp_dir().join(format!("issuer_import_test_{}_{}_{}.json", pid, tid, nanos))
}

#[test]
fn import_private_hex_persists_and_reloads_did_matches() {
    // Path to the compiled binary for this package
    let exe = env!("CARGO_BIN_EXE_issuer_key_tool");

    // Prepare a temp key path and a deterministic 32-byte private key (not secret; test-only)
    let key_path = tmp_key_path();
    let _ = std::fs::remove_file(&key_path);
    let priv_bytes = [0x42u8; 32];
    let priv_hex = hex::encode(priv_bytes);

    // Run the tool to import the private key and print issuer_did
    let output = Command::new(exe)
        .args([
            "--key",
            key_path.to_str().expect("utf8 path"),
            "--import-private-hex",
            &priv_hex,
        ])
        .output()
        .expect("run issuer_key_tool");
    assert!(output.status.success(), "issuer_key_tool exited with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let printed_did = stdout
        .lines()
        .find_map(|l| l.strip_prefix("issuer_did="))
        .expect("issuer_did printed by tool")
        .trim()
        .to_string();

    // Reload the key using the library API and verify DID matches
    let cfg = IssuerConfig { key_path: key_path.clone(), validity_days: 365, did: "did:key:pending".into() };
    let kp = load_or_create_key(&cfg).expect("reload key");
    let derived = derive_did_key_ed25519(&kp.public_key.to_bytes());
    assert_eq!(printed_did, derived, "DID from tool output should match DID derived after reload");

    // Cleanup
    let _ = std::fs::remove_file(&key_path);
}

#[test]
fn import_private_hex_rejects_invalid_hex() {
    let exe = env!("CARGO_BIN_EXE_issuer_key_tool");
    let key_path = tmp_key_path();
    let _ = std::fs::remove_file(&key_path);

    // Too short (not 32 bytes) and non-hex content
    let bad_hex = "zzzz"; // clearly invalid
    let output = Command::new(exe)
        .args(["--key", key_path.to_str().unwrap(), "--import-private-hex", bad_hex])
        .output()
        .expect("run issuer_key_tool");
    assert!(
        !output.status.success(),
        "tool should fail on invalid hex; status={:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Ensure no key file was created
    assert!(
        !key_path.exists(),
        "key file should not be created on invalid import"
    );
}
