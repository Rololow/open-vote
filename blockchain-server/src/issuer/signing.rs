use anyhow::Result;
use serde::Serialize;
use sha2::Digest;
use super::{IssuerKeyPair, UnsignedCitizenCredential, SignedCitizenCredential};
#[cfg(feature="identity")] use common::canonical_json_str;

/// Artéfacts produits lors de la signature d'un credential (diagnostic / réutilisation).
#[derive(Debug, Clone, Serialize)]
pub struct CredentialSignatureArtifacts {
    /// Représentation JSON canonique utilisée pour le calcul de la signature.
    pub canonical_json: String,
    /// Digest SHA-256 (hex) du JSON canonique – peut servir de commitment secondaire.
    pub digest_sha256_hex: String,
    /// Signature Ed25519 encodée en base58btc (sans préfixe multibase ajouté ici).
    pub signature_base58: String,
    /// Méthode de vérification (DID fragment).
    pub verification_method: String,
}

/// Construit la preuve Ed25519 (prototype Data Integrity) d'un credential unsigned.
/// Retourne le credential signé + artéfacts détaillés.
pub fn sign_credential_with_artifacts(
    kp: &IssuerKeyPair,
    unsigned: &UnsignedCitizenCredential,
) -> Result<(SignedCitizenCredential, CredentialSignatureArtifacts)> {
    // 1. Sérialisation & canonicalisation déterministe si disponible
    let raw_json = serde_json::to_string(unsigned)?;
    #[cfg(feature="identity")] let canonical = canonical_json_str(&raw_json)?;
    #[cfg(not(feature="identity"))] let canonical = raw_json;

    // 2. Digest + signature
    let digest = sha2::Sha256::digest(canonical.as_bytes());
    let signature = kp.keypair.sign(canonical.as_bytes());
    let sig_b58 = bs58::encode(signature.to_bytes()).into_string();

    let verification_method = format!("{}#controller", unsigned.issuer);

    // 3. Proof JSON minimaliste
    // Utiliser la date d'émission du credential comme "created" pour un comportement déterministe
    // (évite la non-déterminisme entre deux signatures du même input pendant les tests).
    let proof_created = unsigned.issuanceDate.clone();
    let proof = serde_json::json!({
        "type": "Ed25519Signature2020",
        "created": proof_created,
        "proofPurpose": "assertionMethod",
        "verificationMethod": verification_method,
        "canonicalization": "identity-canonical-json@v0 (sorted-keys, deterministic)",
        "digestAlgorithm": "sha256",
        "digest": hex::encode(&digest),
        "proofValue": format!("z{}", sig_b58),
    });

    let signed = SignedCitizenCredential { unsigned: unsigned.clone(), proof };
    let artifacts = CredentialSignatureArtifacts {
        canonical_json: canonical,
        digest_sha256_hex: hex::encode(digest),
        signature_base58: sig_b58,
        verification_method,
    };
    Ok((signed, artifacts))
}

/// Version simplifiée conservant uniquement l'objet signé (compatibilité).
pub fn sign_credential(kp: &IssuerKeyPair, unsigned: &UnsignedCitizenCredential) -> Result<SignedCitizenCredential> {
    let (signed, _artifacts) = sign_credential_with_artifacts(kp, unsigned)?;
    Ok(signed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issuer::{IssuerKeyPair, UnsignedCitizenCredential};

    fn fixed_unsigned() -> UnsignedCitizenCredential {
        UnsignedCitizenCredential {
            context: vec!["https://www.w3.org/2018/credentials/v1".into()],
            types: vec!["VerifiableCredential".into(), "CitizenCredential".into()],
            issuer: "did:key:zIssuerTest".into(),
            issuanceDate: "2025-09-24T00:00:00Z".into(),
            expirationDate: "2026-09-24T00:00:00Z".into(),
            credentialSubject: serde_json::json!({"id":"did:key:zSubjTest"}),
            metadata: serde_json::json!({"roles":["citizen"],"version":1}),
        }
    }

    #[test]
    fn deterministic_signature_same_input() {
        let kp = IssuerKeyPair::generate();
        let unsigned = fixed_unsigned();
        let (signed1, art1) = sign_credential_with_artifacts(&kp, &unsigned).unwrap();
        let (signed2, art2) = sign_credential_with_artifacts(&kp, &unsigned).unwrap();
        assert_eq!(art1.digest_sha256_hex, art2.digest_sha256_hex, "digest stable");
        assert_eq!(art1.signature_base58, art2.signature_base58, "signature stable for same key & data");
        assert_eq!(serde_json::to_string(&signed1).unwrap(), serde_json::to_string(&signed2).unwrap(), "same signed JSON");
    }
}
