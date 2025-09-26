use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, error};
use chrono::Utc;

mod identity;

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
        /// Hash d'engagement d'identité à référencer dans la transaction (facultatif)
        #[arg(long)]
        identity_hash: Option<String>,
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
            identity_hash,
            key_file,
            node_url,
        } => {
            create_proposal(&title, &description, identity_hash, &key_file, &node_url).await
        }
        Commands::DidGenerate { key_file } => did_generate(&key_file).await,
    Commands::VcRequest { endpoint, subject_did, out_dir } => identity::vc_request(&endpoint, &subject_did, &out_dir).await,
    Commands::VcHash { file } => identity::vc_hash(&file).await,
    Commands::VcShow { file, hash, dir } => identity::vc_show(file, hash, &dir).await,
    Commands::VcCommit { file, hash, dir, node_url, issuer_endpoint } => identity::vc_commit(file, hash, &dir, &node_url, issuer_endpoint.as_deref()).await,
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
    identity_hash: Option<String>,
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

    // 4.1. Attacher une référence d'identité si fournie
    if let Some(h) = identity_hash {
        tx.identity_ref = Some(h.clone());
        println!("🔗 identity_ref attachée: {}", h);
    }

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

// Identity-specific helper functions moved to identity.rs module.