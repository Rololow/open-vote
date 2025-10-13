use std::path::PathBuf;
use blockchain_server::issuer::{IssuerConfig, load_or_create_key, derive_did_key_ed25519, IssuerKeyFile};
use base64::Engine as _;

fn print_usage() {
    eprintln!(
        "issuer_key_tool usage:\n  --key <path>                 Path to issuer key JSON (default: issuer_ed25519_key.json)\n  --export-jwk <path>          Export public JWK to path and print issuer DID\n  --import-private-hex <hex>   Import a 32-byte Ed25519 private key (hex) and persist to --key\n  --help                       Show this help\n"
    );
}

fn main() {
    let mut key_path: Option<PathBuf> = None;
    let mut export_jwk: Option<PathBuf> = None;
    let mut import_priv_hex: Option<String> = None;
    let mut i = 1;
    let args: Vec<String> = std::env::args().collect();
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => { print_usage(); return; }
            "--key" => { i+=1; if i<args.len() { key_path = Some(PathBuf::from(&args[i])); } }
            "--export-jwk" => { i+=1; if i<args.len() { export_jwk = Some(PathBuf::from(&args[i])); } }
            "--import-private-hex" => { i+=1; if i<args.len() { import_priv_hex = Some(args[i].clone()); } }
            other => { eprintln!("Unknown arg: {}", other); print_usage(); return; }
        }
        i+=1;
    }

    let mut cfg = IssuerConfig::from_env();
    if let Some(k) = key_path { cfg.key_path = k; }

    // Determine final keypair: import if requested, else load/create
    let keypair = if let Some(hex_priv) = import_priv_hex {
        // Parse and validate 32-byte hex private key
        let bytes = match hex::decode(hex_priv.trim()) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to decode --import-private-hex: {}", e);
                std::process::exit(2);
            }
        };
        if bytes.len() != 32 {
            eprintln!("--import-private-hex must be 32 bytes (64 hex chars); got {} bytes", bytes.len());
            std::process::exit(2);
        }
        let mut priv_bytes = [0u8; 32];
        priv_bytes.copy_from_slice(&bytes);

        // Build keypair and persist to cfg.key_path
        let kp = crypto_lib::KeyPair::from_private_bytes(&priv_bytes);
        let file = IssuerKeyFile {
            public_key_hex: kp.public_key().to_hex(),
            private_key_hex: hex::encode(kp.private_key_bytes()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let json = match serde_json::to_string_pretty(&file) {
            Ok(j) => j,
            Err(e) => { eprintln!("Serialize key file failed: {}", e); std::process::exit(2); }
        };
        if let Err(e) = std::fs::write(&cfg.key_path, json) {
            eprintln!("Write key file ({}): {}", cfg.key_path.display(), e);
            std::process::exit(2);
        }
        kp
    } else {
        // Load or create an issuer key if not importing
        match load_or_create_key(&cfg) {
            Ok(k) => k.keypair,
            Err(e) => { eprintln!("load_or_create_key failed: {}", e); std::process::exit(1); }
        }
    };

    // Always print issuer DID and optionally export a public JWK
    let pub_bytes = keypair.public_key().to_bytes();
    let issuer_did = derive_did_key_ed25519(&pub_bytes);
    println!("issuer_did={}", issuer_did);

    if let Some(out) = export_jwk {
        let x_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(pub_bytes);
        let pub_hex = hex::encode(pub_bytes);
        let kid = format!("ed25519-{}", &pub_hex[..16]);
        let jwk = serde_json::json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "x": x_b64,
            "kid": kid,
            "issuer_did": issuer_did,
        });
        if let Err(e) = std::fs::write(&out, serde_json::to_string_pretty(&jwk).unwrap()) {
            eprintln!("write JWK failed ({}): {}", out.display(), e);
            std::process::exit(2);
        }
        println!("exported_jwk={}", out.display());
    }
}
