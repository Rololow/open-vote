use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// Crypto
use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, OsRng, generic_array::GenericArray}};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{SaltString, PasswordHash};
use zeroize::Zeroize;

const DEFAULT_STORE: &str = "~/.e-gov-wallet/keys.enc";
const MAGIC: &[u8; 8] = b"EGOVKS\x01"; // format versioned

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeystoreEntry {
    pub id: String,           // key identifier (e.g., hex of public key prefix)
    pub public_key_hex: String,
    pub private_key_hex: String, // 32-byte hex (Ed25519 secret)
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct KeystoreData {
    pub entries: Vec<KeystoreEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Envelope {
    // KDF parameters and metadata
    kdf: KdfParams,
    // AEAD nonce and ciphertext
    nonce: String,      // hex
    ciphertext: String, // hex
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct KdfParams {
    salt: String, // base64 or hex; we use base64 for compatibility
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
}

fn expand_path(p: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(p).to_string())
}

fn default_store_path() -> PathBuf { expand_path(DEFAULT_STORE) }

fn derive_key(passphrase: &str, params: &KdfParams) -> Result<[u8; 32]> {
    let salt_bytes = base64::decode(&params.salt).context("decode salt b64")?;
    let argon = Argon2::new_with_threads(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params.m_cost,
        params.t_cost,
        params.p_cost,
    ).context("argon2 params")?;
    let mut out = [0u8; 32];
    argon.hash_password_into(passphrase.as_bytes(), &salt_bytes, &mut out).context("argon2 kdf")?;
    Ok(out)
}

fn encrypt(passphrase: &str, data: &KeystoreData) -> Result<Vec<u8>> {
    // serialize cleartext JSON
    let plaintext = serde_json::to_vec(data)?;
    // KDF params
    let salt = SaltString::generate(&mut OsRng);
    let params = KdfParams { salt: salt.to_string(), m_cost: 19456, t_cost: 2, p_cost: 1 };
    let key = derive_key(passphrase, &params)?;
    // AEAD
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let mut nonce = [0u8; 12]; rand::thread_rng().fill_bytes(&mut nonce);
    let ct = cipher.encrypt(GenericArray::from_slice(&nonce), plaintext.as_ref()).context("aead encrypt")?;
    // envelope
    let env = Envelope { kdf: params, nonce: hex::encode(nonce), ciphertext: hex::encode(ct) };
    let mut out = Vec::with_capacity(8 + 4 + 4 + env.ciphertext.len());
    out.extend_from_slice(MAGIC);
    let json = serde_json::to_vec_pretty(&env)?;
    // length prefix (u32 LE) for json segment
    let len = json.len() as u32;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&json);
    Ok(out)
}

fn decrypt(passphrase: &str, bytes: &[u8]) -> Result<KeystoreData> {
    // parse header
    if bytes.len() < 12 { anyhow::bail!("keystore file too small"); }
    if &bytes[0..8] != MAGIC { anyhow::bail!("invalid keystore magic/version"); }
    let mut len_bytes = [0u8; 4]; len_bytes.copy_from_slice(&bytes[8..12]);
    let json_len = u32::from_le_bytes(len_bytes) as usize;
    if bytes.len() < 12 + json_len { anyhow::bail!("truncated keystore"); }
    let json = &bytes[12..12+json_len];
    let env: Envelope = serde_json::from_slice(json).context("envelope json")?;
    // derive key
    let key = derive_key(passphrase, &env.kdf)?;
    // decrypt
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = hex::decode(env.nonce).context("nonce hex")?;
    let ct = hex::decode(env.ciphertext).context("ct hex")?;
    let pt = cipher.decrypt(GenericArray::from_slice(&nonce), ct.as_ref()).context("aead decrypt")?;
    let data: KeystoreData = serde_json::from_slice(&pt).context("plaintext json")?;
    Ok(data)
}

fn read_store(path: &Path) -> Result<Vec<u8>> {
    Ok(fs::read(path).with_context(|| format!("read {}", path.display()))?)
}

fn write_store(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    fs::write(path, bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

// Public API

pub fn init_if_absent(passphrase: &str, store: Option<&str>) -> Result<PathBuf> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    if path.exists() { anyhow::bail!("keystore existe déjà: {}", path.display()); }
    let data = KeystoreData::default();
    let bytes = encrypt(passphrase, &data)?;
    write_store(&path, &bytes)?;
    println!("✅ Keystore initialisé: {}", path.display());
    Ok(path)
}

pub fn list(passphrase: &str, store: Option<&str>) -> Result<()> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    let bytes = read_store(&path)?;
    let data = decrypt(passphrase, &bytes)?;
    if data.entries.is_empty() {
        println!("Aucune clé dans le keystore ({})", path.display());
    } else {
        println!("Clés dans {}:", path.display());
        for e in data.entries.iter() {
            println!("- id={} pub={}", e.id, e.public_key_hex);
        }
    }
    Ok(())
}

pub fn import_key(passphrase: &str, private_key_hex: &str, public_key_hex: &str, id: Option<&str>, store: Option<&str>) -> Result<String> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    let mut bytes = read_store(&path)?;
    let mut data = decrypt(passphrase, &bytes)?;
    let key_id = id.map(|s| s.to_string()).unwrap_or_else(|| public_key_hex.chars().take(16).collect());
    // check duplicate
    if data.entries.iter().any(|e| e.id == key_id) { anyhow::bail!("id déjà présent"); }
    data.entries.push(KeystoreEntry { id: key_id.clone(), public_key_hex: public_key_hex.to_string(), private_key_hex: private_key_hex.to_string() });
    bytes.zeroize();
    let new_bytes = encrypt(passphrase, &data)?;
    write_store(&path, &new_bytes)?;
    println!("✅ Importé: id={}", key_id);
    Ok(key_id)
}

pub fn export_key(passphrase: &str, id: &str, store: Option<&str>) -> Result<KeystoreEntry> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    let bytes = read_store(&path)?;
    let data = decrypt(passphrase, &bytes)?;
    let entry = data.entries.into_iter().find(|e| e.id == id).ok_or_else(|| anyhow::anyhow!("id introuvable"))?;
    println!("🔑 Export id={} pub={}", entry.id, entry.public_key_hex);
    Ok(entry)
}

pub fn remove_key(passphrase: &str, id: &str, store: Option<&str>) -> Result<()> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    let bytes = read_store(&path)?;
    let mut data = decrypt(passphrase, &bytes)?;
    let before = data.entries.len();
    data.entries.retain(|e| e.id != id);
    if data.entries.len() == before { anyhow::bail!("id introuvable"); }
    let new_bytes = encrypt(passphrase, &data)?;
    write_store(&path, &new_bytes)?;
    println!("🗑️  Supprimé id={}", id);
    Ok(())
}

pub fn backup(store: Option<&str>, out_path: &str) -> Result<PathBuf> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    let out = expand_path(out_path);
    if let Some(parent) = out.parent() { fs::create_dir_all(parent)?; }
    fs::copy(&path, &out).with_context(|| format!("copie {} -> {}", path.display(), out.display()))?;
    println!("📦 Backup écrit: {}", out.display());
    Ok(out)
}

pub fn restore(store: Option<&str>, backup_file: &str) -> Result<PathBuf> {
    let path = store.map(expand_path).unwrap_or_else(default_store_path);
    let backup_path = expand_path(backup_file);
    if !backup_path.exists() { anyhow::bail!("backup introuvable: {}", backup_path.display()); }
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    fs::copy(&backup_path, &path).with_context(|| format!("restaure {} -> {}", backup_path.display(), path.display()))?;
    println!("✅ Restauré vers {}", path.display());
    Ok(path)
}

// Helper pour charger une clé par id (utilisable par les autres commandes)
pub fn load_private_key(passphrase: &str, id: &str, store: Option<&str>) -> Result<[u8; 32]> {
    let entry = export_key(passphrase, id, store)?;
    let bytes = hex::decode(entry.private_key_hex.trim()).context("decode hex")?;
    if bytes.len() != 32 { anyhow::bail!("clé privée doit faire 32 octets"); }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn hex32_from_byte(b: u8) -> String {
        let v = vec![b; 32];
        hex::encode(v)
    }

    #[test]
    fn keystore_roundtrip_and_wrong_passphrase() {
        let tmp = tempdir().unwrap();
        let store_path = tmp.path().join("keys.enc");
        let store_str = store_path.to_str().unwrap();
        let pass = "passphrase123";
        // init
        init_if_absent(pass, Some(store_str)).expect("init");
        // import
        let priv_hex = hex32_from_byte(0x11);
        let pub_hex = hex32_from_byte(0x22);
        let kid = import_key(pass, &priv_hex, &pub_hex, Some("myid"), Some(store_str)).expect("import");
        assert_eq!(kid, "myid");
        // list (should succeed)
        list(pass, Some(store_str)).expect("list");
        // export
        let entry = export_key(pass, &kid, Some(store_str)).expect("export");
        assert_eq!(entry.id, kid);
        assert_eq!(entry.public_key_hex, pub_hex);
        // load_private_key
        let sk = load_private_key(pass, &kid, Some(store_str)).expect("load");
        assert_eq!(hex::encode(sk), priv_hex);
        // wrong passphrase should fail
        let err = list("wrongpass", Some(store_str)).err().expect("expected error");
        let es = format!("{}", err);
        assert!(es.contains("aead decrypt") || es.contains("invalid keystore magic"), "unexpected error: {}", es);
    }

    #[test]
    fn backup_and_restore() {
        let tmp = tempdir().unwrap();
        let store_path = tmp.path().join("keys.enc");
        let backup_path = tmp.path().join("backup.keys.enc");
        let store_str = store_path.to_str().unwrap();
        let backup_str = backup_path.to_str().unwrap();
        let pass = "passphrase123";
        // init + import
        init_if_absent(pass, Some(store_str)).expect("init");
        let priv_hex = hex32_from_byte(0x33);
        let pub_hex = hex32_from_byte(0x44);
        import_key(pass, &priv_hex, &pub_hex, Some("bku"), Some(store_str)).expect("import");
        // backup
        backup(Some(store_str), backup_str).expect("backup");
        assert!(backup_path.exists());
        // simulate restore to same store path
        restore(Some(store_str), backup_str).expect("restore");
        // verify load still works
        let sk = load_private_key(pass, "bku", Some(store_str)).expect("load");
        assert_eq!(hex::encode(sk), priv_hex);
    }
}
