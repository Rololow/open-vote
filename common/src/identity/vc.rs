use serde::{Serialize, Deserialize};
use ssi::vc::{Credential, VerificationResult};
use crate::identity::Did;

/// Wrapper interne sur un Verifiable Credential citoyen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitizenCredentialWrapper {
    pub raw_credential_json: String,
    pub canonical_hash_hex: Option<String>,
}

impl CitizenCredentialWrapper {
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        let _v: serde_json::Value = serde_json::from_str(raw)?;
        Ok(Self { raw_credential_json: raw.to_string(), canonical_hash_hex: None })
    }

    pub fn issuer_did(&self) -> Option<Did> {
        let v: serde_json::Value = serde_json::from_str(&self.raw_credential_json).ok()?;
        v.get("issuer").and_then(|x| x.as_str()).map(|s| Did(s.to_string()))
    }

    pub fn subject_did(&self) -> Option<Did> {
        let v: serde_json::Value = serde_json::from_str(&self.raw_credential_json).ok()?;
        v.get("credentialSubject").and_then(|cs| cs.get("id")).and_then(|id| id.as_str()).map(|s| Did(s.to_string()))
    }

    /// Stub vérification – pas de résolution DID encore.
    pub fn verify_signature_stub(&self) -> anyhow::Result<VerificationResult> {
        let _cred: Credential = serde_json::from_str(&self.raw_credential_json)?;
        Ok(VerificationResult { errors: vec![], warnings: vec!["did-resolution-disabled (stub)".into()], checks: vec![] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_issuer_subject() {
        let raw = r#"{"@context":["https://www.w3.org/2018/credentials/v1"],"type":["VerifiableCredential"],"issuer":"did:key:zISSUER","credentialSubject":{"id":"did:key:zSUBJ"}}"#;
        let c = CitizenCredentialWrapper::parse(raw).unwrap();
        assert_eq!(c.issuer_did().unwrap().as_str(), "did:key:zISSUER");
        assert_eq!(c.subject_did().unwrap().as_str(), "did:key:zSUBJ");
    }
}
