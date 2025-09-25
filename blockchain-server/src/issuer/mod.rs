use std::{path::PathBuf, fs};
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use tracing::{info, warn};
// use sha2::Digest; // kept for potential future hashing of external metadata

mod signing;
pub use signing::{sign_credential_with_artifacts, CredentialSignatureArtifacts};

/// Configuration runtime de l'issuer.
#[derive(Debug, Clone)]
pub struct IssuerConfig {
    pub key_path: PathBuf,
    pub validity_days: i64,
    pub did: String, // did:key:... dérivé dynamiquement à partir de la clé publique
}

impl IssuerConfig {
    pub fn from_env() -> Self {
        let key_path = std::env::var("ISSUER_KEY_PATH").unwrap_or_else(|_| "issuer_ed25519_key.json".into());
        let validity_days = std::env::var("ISSUER_VC_VALIDITY_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(365);
        // did initial placeholder; sera remplacé après chargement de la clé.
        let did = "did:key:pending".to_string();
        Self { key_path: key_path.into(), validity_days, did }
    }
}

/// Dérive un DID did:key (Ed25519) depuis une clé publique 32 bytes.
/// Spécification: multicodec ed25519-pub = 0xED (237), varint -> 0xED 0x01
/// Multibase base58btc préfixée 'z'.
pub fn derive_did_key_ed25519(pubkey: &[u8;32]) -> String {
    // multicodec prefix ed25519-pub: 0xED 0x01
    let mut data = Vec::with_capacity(34);
    data.push(0xED);
    data.push(0x01);
    data.extend_from_slice(pubkey);
    let multibase = format!("z{}", bs58::encode(data).into_string());
    format!("did:key:{}", multibase)
}

/// Représentation persistée de la clé (simple, pas encore JWK complet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerKeyFile {
    pub public_key_hex: String,
    pub private_key_hex: String,
    pub created_at: String,
}

pub struct IssuerKeyPair {
    pub public_key: crypto_lib::PublicKey,
    pub keypair: crypto_lib::KeyPair,
}

impl IssuerKeyPair {
    pub fn generate() -> Self {
        let kp = crypto_lib::KeyPair::generate();
        Self { public_key: kp.public_key().clone(), keypair: kp }
    }
}

/// Charge ou génère une clé d'issuer sur disque.
pub fn load_or_create_key(cfg: &IssuerConfig) -> Result<IssuerKeyPair> {
    if cfg.key_path.exists() {
        let raw = fs::read_to_string(&cfg.key_path)
            .with_context(|| format!("Lecture clé issuer: {}", cfg.key_path.display()))?;
        let stored: IssuerKeyFile = serde_json::from_str(&raw)?;
        // Restaurer la clé privée depuis le fichier
        let priv_bytes_vec = hex::decode(stored.private_key_hex.trim())
            .with_context(|| "Décodage hex clé privée issuer")?;
        if priv_bytes_vec.len() != 32 {
            anyhow::bail!("Clé privée issuer inattendue: {} bytes (attendu 32)", priv_bytes_vec.len());
        }
        let mut priv_bytes = [0u8; 32];
        priv_bytes.copy_from_slice(&priv_bytes_vec);
        let kp = crypto_lib::KeyPair::from_private_bytes(&priv_bytes);

        // Vérification de cohérence avec la clé publique stockée (si fournie)
        let derived_pub_hex = kp.public_key().to_hex();
        if !stored.public_key_hex.is_empty() && stored.public_key_hex.to_lowercase() != derived_pub_hex {
            warn!(
                "Incohérence clé issuer: public_key_hex stocké ≠ dérivé (stocké={}, dérivé={})",
                stored.public_key_hex, derived_pub_hex
            );
        }

        Ok(IssuerKeyPair { public_key: kp.public_key().clone(), keypair: kp })
    } else {
        info!("Génération nouvelle clé issuer: {}", cfg.key_path.display());
        let kp = crypto_lib::KeyPair::generate();
        let file = IssuerKeyFile {
            public_key_hex: kp.public_key().to_hex(),
            private_key_hex: hex::encode(kp.private_key_bytes()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&file)?;
        fs::write(&cfg.key_path, json)?;
        Ok(IssuerKeyPair { public_key: kp.public_key().clone(), keypair: kp })
    }
}

/// VC minimal à signer (structure interne avant packaging VC complet)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct UnsignedCitizenCredential {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    #[serde(rename = "type")]
    pub types: Vec<String>,
    pub issuer: String,
    pub issuanceDate: String,
    pub expirationDate: String,
    pub credentialSubject: serde_json::Value,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SignedCitizenCredential {
    #[serde(flatten)]
    pub unsigned: UnsignedCitizenCredential,
    pub proof: serde_json::Value, // placeholder
}

pub fn build_unsigned_credential(cfg: &IssuerConfig, subject_did: &str) -> UnsignedCitizenCredential {
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::days(cfg.validity_days);
    // cfg.did peut être placeholder; l'appelant devrait substituer par dérivation réelle.
    UnsignedCitizenCredential {
        context: vec!["https://www.w3.org/2018/credentials/v1".into()],
        types: vec!["VerifiableCredential".into(), "CitizenCredential".into()],
        issuer: cfg.did.clone(),
        issuanceDate: now.to_rfc3339(),
        expirationDate: exp.to_rfc3339(),
        credentialSubject: serde_json::json!({
            "id": subject_did,
            "country": "FR",
            "over18": true
        }),
        metadata: serde_json::json!({
            "roles": ["citizen"],
            "version": 1
        }),
    }
}

// Signature logic moved to signing.rs; legacy stub removed. Public APIs re-exported above.

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_or_create_key_persists_and_reloads() {
        // Utiliser un chemin de fichier temporaire dans le dossier cible
        let tmp_path = std::env::temp_dir().join("issuer_ed25519_key_test.json");
        if tmp_path.exists() { let _ = fs::remove_file(&tmp_path); }

        let cfg = IssuerConfig { key_path: tmp_path.clone(), validity_days: 365, did: "did:key:pending".into() };

        // 1) Première création
        let kp1 = load_or_create_key(&cfg).expect("create key");
        let did1 = derive_did_key_ed25519(&kp1.public_key.to_bytes());

        // 2) Rechargement depuis le fichier existant
        let kp2 = load_or_create_key(&cfg).expect("reload key");
        let did2 = derive_did_key_ed25519(&kp2.public_key.to_bytes());

        assert_eq!(kp1.public_key.to_hex(), kp2.public_key.to_hex(), "La clé publique doit être identique après rechargement");
        assert_eq!(did1, did2, "Le DID dérivé doit rester stable après redémarrage");

        let _ = fs::remove_file(&tmp_path);
    }
}
