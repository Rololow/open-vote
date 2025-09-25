use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "wallet-cli")]
#[command(about = "Client en ligne de commande pour le système e-gouvernement blockchain")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Génère une nouvelle paire de clés Ed25519
    GenerateKeypair {
        /// Chemin où sauvegarder la clé privée
        #[arg(short, long, default_value = "private_key.pem")]
        output: PathBuf,
    },
    /// Charge une paire de clés depuis un fichier
    LoadKeypair {
        /// Chemin vers le fichier de clé privée
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Crée une nouvelle proposition de loi
    CreateProposal {
        /// Titre de la proposition
        #[arg(short, long)]
        title: String,
        /// Description de la proposition
        #[arg(short, long)]
        description: String,
        /// Chemin vers le fichier de clé privée
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
    },
    /// Génère un did:key à partir d'une clé privée Ed25519
    DidGenerate {
        /// Chemin vers le fichier de clé privée (hex)
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
    },
    /// Demande un VC à l'issuer et le sauvegarde localement
    VcRequest {
        /// URL de l'issuer (ex: http://localhost:8080)
        #[arg(long, default_value = "http://localhost:8080")]
        endpoint: String,
        /// DID sujet (did:key:...)
        #[arg(long)]
        subject_did: String,
        /// Dossier de sortie (stockage des VC)
        #[arg(long, default_value = "~/.e-gov-wallet/credentials")]
        out_dir: String,
    },
    /// Calcule le hash d'engagement d'un VC local
    VcHash {
        /// Fichier VC JSON
        #[arg(long)]
        file: PathBuf,
    },
    /// Affiche les métadonnées d'un VC et son commitment hash
    VcShow {
        /// Fichier VC JSON (sinon fournir --hash)
        #[arg(long)]
        file: Option<PathBuf>,
        /// Hash d'un VC (si --file non fourni)
        #[arg(long)]
        hash: Option<String>,
        /// Dossier des VC (si on passe un hash)
        #[arg(long, default_value = "~/.e-gov-wallet/credentials")]
        dir: String,
    },
    /// Vérifie la présence du commitment côté nœud (et optionnellement vérifie le VC côté issuer)
    VcCommit {
        /// Fichier VC JSON (sinon fournir --hash)
        #[arg(long)]
        file: Option<PathBuf>,
        /// Hash d'un VC (si --file non fourni)
        #[arg(long)]
        hash: Option<String>,
        /// Dossier des VC (si on passe un hash)
        #[arg(long, default_value = "~/.e-gov-wallet/credentials")]
        dir: String,
        /// URL du nœud blockchain (pour vérifier la présence du commitment)
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
        /// URL de l'issuer pour vérification facultative (POST /issuer/verify)
        #[arg(long)]
        issuer_endpoint: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialiser le logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateKeypair { output } => {
            generate_keypair(&output).await
        }
        Commands::LoadKeypair { input } => {
            load_keypair(&input).await
        }
        Commands::CreateProposal {
            title,
            description,
            key_file,
            node_url,
        } => {
            create_proposal(&title, &description, &key_file, &node_url).await
        }
        Commands::DidGenerate { key_file } => did_generate(&key_file).await,
        Commands::VcRequest { endpoint, subject_did, out_dir } => vc_request(&endpoint, &subject_did, &out_dir).await,
        Commands::VcHash { file } => vc_hash(&file).await,
        Commands::VcShow { file, hash, dir } => vc_show(file, hash, &dir).await,
        Commands::VcCommit { file, hash, dir, node_url, issuer_endpoint } => vc_commit(file, hash, &dir, &node_url, issuer_endpoint.as_deref()).await,
    }
}

async fn generate_keypair(output_path: &PathBuf) -> Result<()> {
    use crypto_lib::KeyPair;
    use std::fs;

    info!("Génération d'une nouvelle paire de clés...");

    // Générer une nouvelle paire de clés
    let keypair = KeyPair::generate();

    // Sauvegarder la clé privée (format hex pour simplifier)
    let private_key_bytes = keypair.private_key_bytes();
    let private_key_hex = hex::encode(private_key_bytes);
    fs::write(output_path, private_key_hex)?;

    println!("✅ Paire de clés générée avec succès !");
    println!("🔑 Clé publique : {}", keypair.public_key().to_hex());
    println!("💾 Clé privée sauvegardée dans : {}", output_path.display());
    println!("⚠️  GARDEZ VOTRE CLÉ PRIVÉE EN SÉCURITÉ !");

    Ok(())
}

async fn load_keypair(input_path: &PathBuf) -> Result<()> {
    use crypto_lib::KeyPair;
    use std::fs;

    info!("Chargement de la paire de clés depuis {:?}...", input_path);

    // Lire la clé privée
    let private_key_hex = fs::read_to_string(input_path)?;
    let private_key_bytes = hex::decode(private_key_hex.trim())?;
    let mut bytes_array = [0u8; 32];
    bytes_array.copy_from_slice(&private_key_bytes);
    let keypair = KeyPair::from_private_bytes(&bytes_array);

    println!("✅ Clé chargée avec succès !");
    println!("🔑 Clé publique : {}", keypair.public_key().to_hex());

    Ok(())
}

async fn create_proposal(
    title: &str,
    description: &str,
    key_file: &PathBuf,
    node_url: &str,
) -> Result<()> {
    use crypto_lib::KeyPair;
    use common::{Transaction, TransactionType, proposal::Proposal};
    use std::fs;
    use uuid::Uuid;
    use chrono::Utc;
    
    info!("Création d'une proposition de loi...");

    // 1. Charger la clé privée
    let private_key_hex = fs::read_to_string(key_file)?;
    let private_key_bytes = hex::decode(private_key_hex.trim())?;
    let mut bytes_array = [0u8; 32];
    bytes_array.copy_from_slice(&private_key_bytes);
    let keypair = KeyPair::from_private_bytes(&bytes_array);

    println!("🔑 Utilisation de la clé publique : {}", keypair.public_key().to_hex());

    // 2. Créer la proposition
    let proposal = Proposal {
        id: Uuid::new_v4(),
        title: title.to_string(),
        category: "Autre".to_string(), // TODO: permettre de spécifier la catégorie
        description: description.to_string(),
        full_text: description.to_string(), // Pour l'instant, même contenu
        estimated_budget: None,
        implementation_timeline: None,
        tags: vec![],
        author_id: Some(keypair.public_key().to_hex()),
        author_name: Some("Wallet CLI".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(30),
        status: common::proposal::ProposalStatus::CollectingSignatures,
        supporters_count: 0,
    };

    // 3. Créer la transaction
    let mut tx = Transaction::new(
        TransactionType::CreateProposal(proposal.clone()),
        keypair.public_key().clone(),
        keypair.sign(b"temp"), // Signature temporaire
        0,
        0,
    );

    // 4. Signer la transaction correctement
    let sign_msg = format!(
        "TRANSACTION:{}:{}:{}",
        tx.id,
        tx.timestamp,
        tx.data_hash.to_hex()
    );
    tx.signature = keypair.sign(sign_msg.as_bytes());

    println!("📝 Proposition créée : {}", proposal.title);
    println!("🆔 ID de la transaction : {}", tx.id);

    // 5. Envoyer la transaction au nœud blockchain
    let client = reqwest::Client::new();
    let rpc_url = format!("{}/rpc/broadcast_transaction", node_url);
    
    let request_body = serde_json::json!({
        "transaction": tx
    });

    println!("🚀 Envoi de la transaction vers : {}", rpc_url);
    
    let response = client
        .post(&rpc_url)
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        println!("✅ Transaction envoyée avec succès !");
        println!("📋 Réponse du nœud : {}", serde_json::to_string_pretty(&result)?);
    } else {
        let error_text = response.text().await?;
        error!("❌ Erreur lors de l'envoi : {}", error_text);
        return Err(anyhow::anyhow!("Échec de l'envoi de la transaction"));
    }

    Ok(())
}

async fn did_generate(key_file: &PathBuf) -> Result<()> {
    use crypto_lib::KeyPair;
    use base64ct::{Base64UrlUnpadded, Encoding};

    let priv_hex = std::fs::read_to_string(key_file)?;
    let priv_bytes_vec = hex::decode(priv_hex.trim())?;
    if priv_bytes_vec.len() != 32 { anyhow::bail!("La clé privée doit faire 32 octets hex"); }
    let mut priv_bytes = [0u8;32]; priv_bytes.copy_from_slice(&priv_bytes_vec);
    let kp = KeyPair::from_private_bytes(&priv_bytes);
    let pk = kp.public_key().to_bytes();

    // did:key derivation (multicodec 0xED 0x01 + base58btc prefixed z)
    let mut data = Vec::with_capacity(34); data.push(0xED); data.push(0x01); data.extend_from_slice(&pk);
    let did = format!("did:key:z{}", bs58::encode(data).into_string());
    println!("did:key = {}", did);
    // Also print JWK x for convenience
    println!("jwk.x = {}", Base64UrlUnpadded::encode_string(&pk));
    Ok(())
}

async fn vc_request(endpoint: &str, subject_did: &str, out_dir: &str) -> Result<()> {
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

async fn vc_hash(file: &PathBuf) -> Result<()> {
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

async fn vc_show(file: Option<PathBuf>, hash: Option<String>, dir: &str) -> Result<()> {
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

async fn vc_commit(
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