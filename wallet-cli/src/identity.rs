use anyhow::Result;
use chrono::{DateTime, Utc};
use tracing::info;
use std::path::PathBuf;

fn expand_dir(dir: &str) -> PathBuf {
    let expanded = shellexpand::tilde(dir).to_string();
    PathBuf::from(expanded)
}

fn load_vc_from(file: &PathBuf) -> Result<serde_json::Value> {
    let raw = std::fs::read_to_string(file)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(v)
}

fn vc_metadata(cred: &serde_json::Value) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let issuer = cred.get("issuer").and_then(|v| v.as_str()).map(|s| s.to_string());
    let subject = cred
        .get("credentialSubject")
        .and_then(|cs| cs.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let issuance = cred.get("issuanceDate").and_then(|v| v.as_str()).map(|s| s.to_string());
    let expiration = cred.get("expirationDate").and_then(|v| v.as_str()).map(|s| s.to_string());
    (issuer, subject, issuance, expiration)
}

fn vc_compute_hash(cred: &serde_json::Value) -> Result<String> {
    let mut unsigned = cred.clone();
    if let Some(obj) = unsigned.as_object_mut() { obj.remove("proof"); }
    let raw = serde_json::to_string(&unsigned)?;
    let canonical = common::canonical_json_str(&raw)?;
    let wrapper = common::CitizenCredentialWrapper { raw_credential_json: canonical, canonical_hash_hex: None };
    let digest = common::compute_credential_hash(&wrapper)?;
    Ok(common::hash_hex(&digest))
}

pub fn vc_fail_fast_check_dates(cred: &serde_json::Value) -> Result<()> {
    // issuanceDate required
    let issued_at_str = cred
        .get("issuanceDate")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("issuanceDate manquante dans le VC"))?;
    let issued_at_dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(issued_at_str)
        .map_err(|e| anyhow::anyhow!("issuanceDate invalide: {e}"))?;
    let now_utc = Utc::now();
    if issued_at_dt.with_timezone(&Utc) > now_utc + chrono::Duration::minutes(5) {
        anyhow::bail!("credential not yet valid (issuanceDate in future)");
    }

    if let Some(exp_str) = cred.get("expirationDate").and_then(|v| v.as_str()) {
        let exp_dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(exp_str)
            .map_err(|e| anyhow::anyhow!("expirationDate invalide: {e}"))?;
        if exp_dt.with_timezone(&Utc) < now_utc {
            anyhow::bail!("credential expired");
        }
        if exp_dt <= issued_at_dt {
            anyhow::bail!("expirationDate doit être > issuanceDate");
        }
    }
    Ok(())
}

pub async fn vc_request(endpoint: &str, subject_did: &str, out_dir: &str) -> Result<()> {
    let url = format!("{}/issuer/credential", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "subject_did": subject_did });
    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() { anyhow::bail!("Requête VC échouée: {}", resp.text().await?); }
    let v: serde_json::Value = resp.json().await?;
    let cred = v.get("credential").cloned().ok_or_else(|| anyhow::anyhow!("payload inattendu"))?;
    // compute commitment hash using common::identity
    let mut cred_no_proof = cred.clone();
    if let Some(obj) = cred_no_proof.as_object_mut() { obj.remove("proof"); }
    let raw = serde_json::to_string(&cred_no_proof)?;
    let canonical = common::canonical_json_str(&raw)?;
    let wrapper = common::CitizenCredentialWrapper { raw_credential_json: canonical.clone(), canonical_hash_hex: None };
    let digest = common::compute_credential_hash(&wrapper)?;
    let hash_hex = common::hash_hex(&digest);

    // storage path
    let expanded = shellexpand::tilde(out_dir).to_string();
    std::fs::create_dir_all(&expanded)?;
    let path = std::path::Path::new(&expanded).join(format!("{}.json", hash_hex));
    std::fs::write(&path, serde_json::to_string_pretty(&cred)?)?;
    println!("✅ VC sauvegardé: {}", path.display());
    println!("🔗 commitment_hash: {}", hash_hex);
    Ok(())
}

pub async fn vc_hash(file: &PathBuf) -> Result<()> {
    let raw_json = std::fs::read_to_string(file)?;
    let value: serde_json::Value = serde_json::from_str(&raw_json)?;
    let mut unsigned = value.clone();
    if let Some(obj) = unsigned.as_object_mut() { obj.remove("proof"); }
    let raw = serde_json::to_string(&unsigned)?;
    let canonical = common::canonical_json_str(&raw)?;
    let wrapper = common::CitizenCredentialWrapper { raw_credential_json: canonical, canonical_hash_hex: None };
    let digest = common::compute_credential_hash(&wrapper)?;
    println!("commitment_hash = {}", common::hash_hex(&digest));
    Ok(())
}

pub async fn vc_show(file: Option<PathBuf>, hash: Option<String>, dir: &str) -> Result<()> {
    let cred_file = match (file, hash) {
        (Some(f), _) => f,
        (None, Some(h)) => expand_dir(dir).join(format!("{}.json", h)),
        (None, None) => anyhow::bail!("Fournir --file ou --hash"),
    };
    let cred = load_vc_from(&cred_file)?;
    let (issuer, subject, issuance, expiration) = vc_metadata(&cred);
    let hash_hex = vc_compute_hash(&cred)?;
    println!("Fichier: {}", cred_file.display());
    println!("commitment_hash: {}", hash_hex);
    println!("issuer: {}", issuer.unwrap_or_default());
    println!("subject: {}", subject.unwrap_or_default());
    println!("issuanceDate: {}", issuance.unwrap_or_default());
    println!("expirationDate: {}", expiration.unwrap_or_default());
    Ok(())
}

pub async fn vc_commit(
    file: Option<PathBuf>,
    hash: Option<String>,
    dir: &str,
    node_url: &str,
    issuer_endpoint: Option<&str>,
) -> Result<()> {
    let cred_file_opt = match (&file, &hash) {
        (Some(f), _) => Some(f.clone()),
        (None, Some(h)) => Some(expand_dir(dir).join(format!("{}.json", h))),
        (None, None) => None,
    };

    let (hash_hex, cred_opt) = if let Some(f) = cred_file_opt {
        let cred = load_vc_from(&f)?;
        // Fail-fast: validate dates locally (issuance/expiration) before any network call
        vc_fail_fast_check_dates(&cred)?;
        (vc_compute_hash(&cred)?, Some(cred))
    } else if let Some(h) = hash { (h, None) } else {
        anyhow::bail!("Fournir --file ou --hash");
    };

    // 1) Vérifier présence côté nœud (lecture publique)
    let get_url = format!(
        "{}/identity/commitments/{}",
        node_url.trim_end_matches('/'),
        hash_hex
    );
    let client = reqwest::Client::new();
    let resp = client.get(&get_url).send().await?;
    if resp.status().is_success() {
        println!("✅ Commitment déjà présent côté nœud: {}", hash_hex);
        return Ok(());
    }
    if resp.status().as_u16() == 404 {
        println!("ℹ️  Commitment introuvable côté nœud: {}", hash_hex);
    } else {
        println!("⚠️  Erreur lors de la vérification côté nœud: {}", resp.status());
    }

    // 2) Optionnel: vérifier cryptographiquement via issuer (/issuer/verify)
    if let (Some(endpoint), Some(cred)) = (issuer_endpoint, cred_opt.clone()) {
        let verify_url = format!("{}/issuer/verify", endpoint.trim_end_matches('/'));
        let payload = serde_json::json!({ "credential": cred });
        let vresp = client.post(&verify_url).json(&payload).send().await?;
        if vresp.status().is_success() {
            let body: serde_json::Value = vresp.json().await?;
            println!(
                "Vérification issuer: valid={} issuer_did={} subject={} commitment_hash={}",
                body.get("valid").and_then(|v| v.as_bool()).unwrap_or(false),
                body.get("issuer_did").and_then(|v| v.as_str()).unwrap_or(""),
                body.get("subject_did").and_then(|v| v.as_str()).unwrap_or(""),
                body.get("commitment_hash").and_then(|v| v.as_str()).unwrap_or("")
            );
        } else {
            println!("⚠️  Échec vérification issuer: {}", vresp.status());
        }
    } else {
        println!("Astuce: fournissez --issuer-endpoint et --file pour une vérification cryptographique (facultative).");
    }

    // 3) Si le commitment est absent et que l'utilisateur a fourni --issuer-endpoint et --file, tenter le POST /identity/commit
    if let (Some(_endpoint_for_check), Some(cred)) = (issuer_endpoint, cred_opt.as_ref()) {
        let post_url = format!("{}/identity/commit", node_url.trim_end_matches('/'));
        println!("➡️  Tentative d'enregistrement du commitment via POST {}", post_url);
        let payload = serde_json::json!({ "credential": cred });
        let presp = client.post(&post_url).json(&payload).send().await?;
        if presp.status().is_success() {
            let body: serde_json::Value = presp.json().await.unwrap_or(serde_json::json!({}));
            println!(
                "✅ Enregistrement effectué (id={}, existed={})",
                body.get("id").and_then(|v| v.as_i64()).unwrap_or_default(),
                body.get("existed").and_then(|v| v.as_bool()).unwrap_or(false)
            );
            // Re-vérifier la présence
            let reget = client.get(&get_url).send().await?;
            if reget.status().is_success() {
                println!("✅ Commitment désormais présent côté nœud: {}", hash_hex);
                return Ok(());
            } else {
                println!("⚠️  Enregistrement signalé réussi mais GET confirme pas encore (status: {})", reget.status());
                return Err(anyhow::anyhow!("commitment non visible après enregistrement"));
            }
        } else {
            let status = presp.status();
            let err_text = presp.text().await.unwrap_or_default();
            println!("❌ Échec POST /identity/commit: {}\n{}", status, err_text);
            return Err(anyhow::anyhow!("échec enregistrement commitment"));
        }
    }

    // Sinon, échouer en guidant l'utilisateur
    println!("❌ Commitment non enregistré côté nœud.");
    println!("   Recommandé: relancer avec --file et --issuer-endpoint pour permettre l'enregistrement (POST /identity/commit).");
    Err(anyhow::anyhow!("commitment absent côté nœud"))
}
