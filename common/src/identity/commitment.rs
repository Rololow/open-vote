use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use hex::ToHex;
use crate::identity::CitizenCredentialWrapper;

/// Résultat standard d'une opération identité.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityOpResult<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> IdentityOpResult<T> {
    pub fn success(data: T) -> Self { Self { ok: true, data: Some(data), error: None } }
    pub fn failure(msg: impl Into<String>) -> Self { Self { ok: false, data: None, error: Some(msg.into()) } }
}

/// Placeholder pour accumulation future (Merkle / set) – Phase 3.
#[derive(Debug, Default)]
pub struct CommitmentAccumulator { pub count: usize }
impl CommitmentAccumulator { pub fn add(&mut self, _hash: &[u8;32]) { self.count += 1; } }

/// Canonisation simple :
/// 1. Parse JSON
/// 2. Trie récursif objets par clé
/// 3. Émet JSON compact stable (sans espaces)
pub fn canonical_json_str(raw: &str) -> anyhow::Result<String> {
    let v: serde_json::Value = serde_json::from_str(raw)?;
    Ok(canonicalize_value(&v))
}

fn canonicalize_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push('"'); out.push_str(k); out.push_str("\":");
                out.push_str(&canonicalize_value(&map[*k]));
            }
            out.push('}');
            out
        }
        serde_json::Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(canonicalize_value).collect();
            format!("[{}]", parts.join(","))
        }
        _ => v.to_string(),
    }
}

/// Calcule le hash sha256 du JSON canonique d'un credential (sans modification préalable).
pub fn compute_credential_hash(cred: &CitizenCredentialWrapper) -> anyhow::Result<[u8;32]> {
    let canonical = canonical_json_str(&cred.raw_credential_json)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8;32];
    out.copy_from_slice(&digest);
    Ok(out)
}

pub fn hash_hex(bytes: &[u8;32]) -> String { bytes.encode_hex::<String>() }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn canonical_deterministic_objects() {
        let a = r#"{"b":1,"a":2}"#;
        let b = r#"{"a":2,"b":1}"#;
        assert_eq!(canonical_json_str(a).unwrap(), canonical_json_str(b).unwrap());
    }

    #[test]
    fn hash_stable_reordered() {
    let a = r#"{"issuer":"did:key:z","credentialSubject":{"id":"did:key:subj"}}"#;
    let b = r#"{"credentialSubject":{"id":"did:key:subj"},"issuer":"did:key:z"}"#;
        let cred_a = CitizenCredentialWrapper { raw_credential_json: a.into(), canonical_hash_hex: None };
        let cred_b = CitizenCredentialWrapper { raw_credential_json: b.into(), canonical_hash_hex: None };
        let ha = compute_credential_hash(&cred_a).unwrap();
        let hb = compute_credential_hash(&cred_b).unwrap();
        assert_eq!(ha, hb);
    }

    #[test]
    fn golden_hash_example_vc() {
        // Path relative to workspace root: docs/examples/vc_citizen.json
        // During test execution current dir is crate root; traverse up if necessary.
        let candidates = [
            Path::new("../../docs/examples/vc_citizen.json"), // workspace relative when in common/src/identity
            Path::new("../docs/examples/vc_citizen.json"),     // alternative if layout changes
            Path::new("docs/examples/vc_citizen.json"),        // direct (for future moves)
        ];
        let mut content = None;
        for p in candidates.iter() {
            if p.exists() { content = Some(fs::read_to_string(p).expect("read example vc")); break; }
        }
        let vc = content.expect("example VC file not found");
        let wrapper = CitizenCredentialWrapper { raw_credential_json: vc, canonical_hash_hex: None };
        let hash = compute_credential_hash(&wrapper).expect("hash");
        let hex = hash_hex(&hash);
        // First time establishment: if this changes unexpectedly, investigation required.
        // Expected value computed 2025-09-24 with current canonicalization.
    // NOTE: If this value changes, it indicates a canonicalization logic change or source VC modification.
    let expected = "b23fd1575eee282ac689ada455ec16cd604bc008989807ade5b89715688f43b2";
        assert_eq!(hex, expected, "golden hash mismatch: got {hex}");
    }
}
