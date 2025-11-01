//! wallet-cli — command-line wallet utility for the e-government blockchain.
//!
//! Provides simple commands to generate keys, interact with the node API,
//! request VCs from an issuer and produce (mock) anonymous actions for demo
//! purposes. Some ZKP-related commands are available behind the
//! `zkp_groth16` feature.
//!
//! Example:
//!
//! ```no_run
//! wallet-cli generate-keypair --output private_key.pem
//! ```

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, error};

mod identity;
mod keystore;
#[cfg(any(feature = "zkp_groth16", feature = "zkp_halo2"))]
mod zkp;
#[cfg(feature = "zkp_halo2")]
mod zkp_halo2;

// Helper: resolve secret key material from keystore (preferred) or fallback file
fn resolve_secret(
    key_file: &PathBuf,
    key_id: &Option<String>,
    passphrase: &Option<String>,
    store: &Option<String>,
) -> anyhow::Result<[u8; 32]> {
    if let Some(kid) = key_id {
        let pass = match passphrase {
            Some(p) => p.clone(),
            None => std::env::var("WALLET_PASSPHRASE").map_err(|_| anyhow::anyhow!("passphrase requise: fournir --passphrase ou la variable d'environnement WALLET_PASSPHRASE"))?,
        };
        return keystore::load_private_key(&pass, kid, store.as_deref());
    }
    // Fallback fichier clé hex 32 octets
    let private_key_hex = std::fs::read_to_string(key_file)?;
    let private_key_bytes = hex::decode(private_key_hex.trim())?;
    if private_key_bytes.len() != 32 { anyhow::bail!("La clé privée doit faire 32 octets hex"); }
    let mut bytes_array = [0u8; 32];
    bytes_array.copy_from_slice(&private_key_bytes);
    Ok(bytes_array)
}

#[derive(Parser)]
#[command(name = "wallet-cli")]
    #[command(about = "Command-line client for the e-government blockchain system")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Keystore chiffré
    KeystoreInit {
        /// Passphrase (attention: privilégier la saisie via variable d'environnement en CI)
        #[arg(long)]
        passphrase: String,
        /// Chemin du fichier keystore (défaut: ~/.e-gov-wallet/keys.enc)
        #[arg(long)]
        store: Option<String>,
    },
    KeystoreList {
        #[arg(long)]
        passphrase: String,
        #[arg(long)]
        store: Option<String>,
    },
    KeystoreImport {
        #[arg(long)]
        passphrase: String,
        #[arg(long)]
        private_key_hex: String,
        #[arg(long)]
        public_key_hex: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        store: Option<String>,
    },
    KeystoreExport {
        #[arg(long)]
        passphrase: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        store: Option<String>,
    },
    KeystoreRemove {
        #[arg(long)]
        passphrase: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        store: Option<String>,
    },
    KeystoreBackup {
        /// Copie du keystore chiffré vers un fichier cible
        #[arg(long)]
        out: String,
        /// Chemin du keystore source
        #[arg(long)]
        store: Option<String>,
    },
    KeystoreRestore {
        /// Fichier de backup à restaurer
        #[arg(long)]
        backup: String,
        #[arg(long)]
        store: Option<String>,
    },
    /// Génère une paire de clés et l'importe dans le keystore (sans fichier en clair)
    KeystoreGenerateKeypair {
        /// Passphrase du keystore
        #[arg(long)]
        passphrase: String,
        /// Identifiant optionnel (défaut: préfixe de la clé publique)
        #[arg(long)]
        id: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Sortie JSON machine-readable
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Génère une nouvelle paire de clés Ed25519
    GenerateKeypair {
        /// Chemin où sauvegarder la clé privée
        #[arg(short, long, default_value = "private_key.pem")]
        output: PathBuf,
    },
    /// Affiche la racine d'identité courante du nœud
    IdentityRoot {
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
    },
    /// Soumet un support anonyme (mock) avec nullifier dérivé localement
    AnonymousSupportMock {
        /// ID de la proposition (UUID)
        #[arg(long)]
        proposal_id: String,
        /// Fichier de clé privée (32 octets hex) pour dériver un secret stable
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
        /// Identifiant de clé dans le keystore (prend le pas sur --key-file)
        #[arg(long)]
        key_id: Option<String>,
        /// Passphrase du keystore (sinon WALLET_PASSPHRASE)
        #[arg(long)]
        passphrase: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
    },
    /// Soumet un vote anonyme (mock) avec nullifier dérivé localement
    AnonymousVoteMock {
        /// ID de la loi (UUID)
        #[arg(long)]
        law_id: String,
        /// Fichier de clé privée (32 octets hex) pour dériver un secret stable
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
        /// Identifiant de clé dans le keystore (prend le pas sur --key-file)
        #[arg(long)]
        key_id: Option<String>,
        /// Passphrase du keystore (sinon WALLET_PASSPHRASE)
        #[arg(long)]
        passphrase: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
    },
    /// Génére des clés VK/PK de démonstration pour la preuve Groth16 (placeholder)
    #[cfg(feature = "zkp_groth16")]
    ZkpSetupKeys {
        /// Répertoire de données (doit être le même que le nœud utilise)
        #[arg(long, default_value = "./data")]
        data_dir: String,
        /// Version de VK/PK à écrire
        #[arg(long, default_value_t = 1)]
        vk_version: u32,
    },
    /// Soumet un support anonyme avec une vraie preuve Groth16 (placeholder circuit)
    #[cfg(feature = "zkp_groth16")]
    AnonymousSupport {
        /// ID de la proposition (UUID)
        #[arg(long)]
        proposal_id: String,
        /// Chemin de la clé privée (32 octets hex)
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
        /// Identifiant de clé dans le keystore (prend le pas sur --key-file)
        #[arg(long)]
        key_id: Option<String>,
        /// Passphrase du keystore (sinon WALLET_PASSPHRASE)
        #[arg(long)]
        passphrase: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Fichier VC JSON pour calculer le commitment
        #[arg(long)]
        vc_file: PathBuf,
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
        /// Répertoire de données contenant pk/vk
        #[arg(long, default_value = "./data")]
        data_dir: String,
        /// Version VK/PK à utiliser
        #[arg(long, default_value_t = 1)]
        vk_version: u32,
    },
    /// Soumet un vote anonyme avec une vraie preuve Groth16 (placeholder circuit)
    #[cfg(feature = "zkp_groth16")]
    AnonymousVote {
        /// ID de la loi (UUID)
        #[arg(long)]
        law_id: String,
        /// Chemin de la clé privée (32 octets hex)
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
        /// Identifiant de clé dans le keystore (prend le pas sur --key-file)
        #[arg(long)]
        key_id: Option<String>,
        /// Passphrase du keystore (sinon WALLET_PASSPHRASE)
        #[arg(long)]
        passphrase: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Fichier VC JSON pour calculer le commitment
        #[arg(long)]
        vc_file: PathBuf,
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
        /// Répertoire de données contenant pk/vk
        #[arg(long, default_value = "./data")]
        data_dir: String,
        /// Version VK/PK à utiliser
        #[arg(long, default_value_t = 1)]
        vk_version: u32,
    },
    /// Produce and save a Groth16 proof to disk (for node consumption)
    #[cfg(feature = "zkp_groth16")]
    ZkpProve {
        /// Data dir containing pk/vk/poseidon params
        #[arg(long, default_value = "./data")]
        data_dir: String,
        /// VK/PK version to use
        #[arg(long, default_value_t = 1)]
        vk_version: u32,
        /// Public root (32-byte hex)
        #[arg(long)]
        root_hex: String,
        /// Scope string
        #[arg(long)]
        scope: String,
        /// Nullifier hex (32-byte hex)
        #[arg(long)]
        nullifier_hex: String,
        /// Leaf hex (32-byte hex)
        #[arg(long)]
        leaf_hex: String,
        /// Secret hex (32-byte hex)
        #[arg(long)]
        secret_hex: String,
        /// Merkle path siblings as repeated --sib <hex>
        #[arg(long = "sib", num_args = 1..)]
        merkle_path: Vec<String>,
        /// Directions string (e.g. 0101... '0' for left, '1' for right)
        #[arg(long)]
        directions: String,
        /// Output JSON file path to write proof envelope
        #[arg(long, default_value = "zkp_proof.json")]
        out: PathBuf,
    },
    /// Generate and save Poseidon parameters to wallet-cli/zkp/poseidon_params.bin
    #[cfg(feature = "zkp_groth16")]
    ZkpGenPoseidonParams {
        /// Data dir for wallet-cli (where zkp/poseidon_params.bin will be written)
        #[arg(long, default_value = "./wallet-cli")]
        data_dir: String,
    },
    /// Produce a minimal Halo2 proof (POC)
    #[cfg(feature = "zkp_halo2")]
    Halo2Prove {
        /// Output path for POC proof JSON
        #[arg(long, default_value = "halo2_poc.json")]
        out: PathBuf,
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
        /// Identifiant de clé dans le keystore (prend le pas sur --key-file)
        #[arg(long)]
        key_id: Option<String>,
        /// Passphrase du keystore (sinon WALLET_PASSPHRASE)
        #[arg(long)]
        passphrase: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Adresse du nœud blockchain
        #[arg(long, default_value = "http://localhost:3000")]
        node_url: String,
        /// Sortie JSON machine-readable
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Génère un did:key à partir d'une clé privée Ed25519
    DidGenerate {
        /// Chemin vers le fichier de clé privée (hex)
        #[arg(short, long, default_value = "private_key.pem")]
        key_file: PathBuf,
        /// Identifiant de clé dans le keystore (prend le pas sur --key-file)
        #[arg(long)]
        key_id: Option<String>,
        /// Passphrase du keystore (sinon WALLET_PASSPHRASE)
        #[arg(long)]
        passphrase: Option<String>,
        /// Chemin du keystore
        #[arg(long)]
        store: Option<String>,
        /// Sortie JSON machine-readable
        #[arg(long, default_value_t = false)]
        json: bool,
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
    /// Liste les VC locaux dans un dossier
    VcList {
        /// Dossier des VC
        #[arg(long, default_value = "~/.e-gov-wallet/credentials")]
        dir: String,
    },
    /// Révocation locale (suppression fichier): par --file ou --hash
    VcRevokeLocal {
        /// Fichier VC JSON (sinon fournir --hash)
        #[arg(long)]
        file: Option<PathBuf>,
        /// Hash d'un VC (si --file non fourni)
        #[arg(long)]
        hash: Option<String>,
        /// Dossier des VC (si on passe un hash)
        #[arg(long, default_value = "~/.e-gov-wallet/credentials")]
        dir: String,
        /// Confirmer la suppression sans invite
        #[arg(long, default_value_t = true)]
        yes: bool,
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
        // keystore
        Commands::KeystoreInit { passphrase, store } => {
            keystore::init_if_absent(&passphrase, store.as_deref())?; Ok(())
        }
        Commands::KeystoreList { passphrase, store } => {
            keystore::list(&passphrase, store.as_deref())
        }
        Commands::KeystoreImport { passphrase, private_key_hex, public_key_hex, id, store } => {
            keystore::import_key(&passphrase, &private_key_hex, &public_key_hex, id.as_deref(), store.as_deref()).map(|_| ())
        }
        Commands::KeystoreExport { passphrase, id, store } => {
            let e = keystore::export_key(&passphrase, &id, store.as_deref())?; 
            println!("{}", serde_json::to_string_pretty(&e)?);
            Ok(())
        }
        Commands::KeystoreRemove { passphrase, id, store } => {
            keystore::remove_key(&passphrase, &id, store.as_deref())
        }
        Commands::KeystoreBackup { out, store } => {
            keystore::backup(store.as_deref(), &out).map(|_| ())
        }
        Commands::KeystoreRestore { backup, store } => {
            keystore::restore(store.as_deref(), &backup).map(|_| ())
        }
        Commands::KeystoreGenerateKeypair { passphrase, id, store, json } => {
            use crypto_lib::KeyPair;
            // Génération
            let kp = KeyPair::generate();
            let priv_hex = hex::encode(kp.private_key_bytes());
            let pub_hex = kp.public_key().to_hex();
            // Import dans keystore
            let kid = keystore::import_key(&passphrase, &priv_hex, &pub_hex, id.as_deref(), store.as_deref())?;
            // did:key
            let pk = kp.public_key().to_bytes();
            let mut data = Vec::with_capacity(34); data.push(0xED); data.push(0x01); data.extend_from_slice(&pk);
            let did = format!("did:key:z{}", bs58::encode(data).into_string());
            if json {
                let out = serde_json::json!({"id": kid, "public_key": pub_hex, "did": did});
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("✅ Généré et importé: id={} pub={} did={}", kid, pub_hex, did);
                println!("Astuce: utilisez --key-id {} avec vos commandes de signature.", kid);
            }
            Ok(())
        }
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
            key_id,
            passphrase,
            store,
            node_url,
            json,
        } => {
            create_proposal(&title, &description, identity_hash, &key_file, &key_id, &passphrase, &store, &node_url, json).await
        }
        Commands::DidGenerate { key_file, key_id, passphrase, store, json } => did_generate(&key_file, &key_id, &passphrase, &store, json).await,
        Commands::VcRequest { endpoint, subject_did, out_dir } => identity::vc_request(&endpoint, &subject_did, &out_dir).await,
        Commands::VcHash { file } => identity::vc_hash(&file).await,
        Commands::VcShow { file, hash, dir } => identity::vc_show(file, hash, &dir).await,
        Commands::VcList { dir } => identity::vc_list(&dir).await,
        Commands::VcRevokeLocal { file, hash, dir, yes } => identity::vc_revoke_local(file, hash, &dir, yes).await,
        Commands::VcCommit { file, hash, dir, node_url, issuer_endpoint } => identity::vc_commit(file, hash, &dir, &node_url, issuer_endpoint.as_deref()).await,
        Commands::IdentityRoot { node_url } => identity_root(&node_url).await,
        Commands::AnonymousSupportMock { proposal_id, key_file, key_id, passphrase, store, node_url } => anonymous_support_mock(&proposal_id, &key_file, &key_id, &passphrase, &store, &node_url).await,
        Commands::AnonymousVoteMock { law_id, key_file, key_id, passphrase, store, node_url } => anonymous_vote_mock(&law_id, &key_file, &key_id, &passphrase, &store, &node_url).await,
        #[cfg(feature = "zkp_groth16")]
        Commands::ZkpSetupKeys { data_dir, vk_version } => zkp_setup_keys(&data_dir, vk_version).await,
        #[cfg(feature = "zkp_groth16")]
        Commands::AnonymousSupport { proposal_id, key_file, key_id, passphrase, store, vc_file, node_url, data_dir, vk_version } => anonymous_support_real(&proposal_id, &key_file, &key_id, &passphrase, &store, &vc_file, &node_url, &data_dir, vk_version).await,
        #[cfg(feature = "zkp_groth16")]
        Commands::AnonymousVote { law_id, key_file, key_id, passphrase, store, vc_file, node_url, data_dir, vk_version } => anonymous_vote_real(&law_id, &key_file, &key_id, &passphrase, &store, &vc_file, &node_url, &data_dir, vk_version).await,
        #[cfg(feature = "zkp_groth16")]
        Commands::ZkpProve { data_dir, vk_version, root_hex, scope, nullifier_hex, leaf_hex, secret_hex, merkle_path, directions, out } => zkp_prove_cli(&data_dir, vk_version, &root_hex, &scope, &nullifier_hex, &leaf_hex, &secret_hex, &merkle_path, &directions, &out).await,
    #[cfg(feature = "zkp_groth16")]
    Commands::ZkpGenPoseidonParams { data_dir } => zkp_gen_poseidon_params(&data_dir),
        #[cfg(feature = "zkp_halo2")]
        Commands::Halo2Prove { out } => zkp_halo2::halo2_prove(&out).await,
    }
}

#[cfg(any(feature = "zkp_groth16", feature = "zkp_halo2"))]
fn zkp_gen_poseidon_params(data_dir: &str) -> Result<()> {
    use crate::zkp::generate_and_save_poseidon_params;
    generate_and_save_poseidon_params(data_dir)?;
    println!("✅ Poseidon parameters generated and saved to {}/zkp/poseidon_params.bin", data_dir);
    Ok(())
}

#[cfg(feature = "zkp_groth16")]
async fn zkp_prove_cli(
    data_dir: &str,
    vk_version: u32,
    root_hex: &str,
    scope: &str,
    nullifier_hex: &str,
    leaf_hex: &str,
    secret_hex: &str,
    merkle_path: &Vec<String>,
    directions: &str,
    out: &PathBuf,
) -> Result<()> {
    use crate::zkp::prove_membership_nullifier;
    use std::fs::File;
    use std::io::Write;

    println!("Producing proof (this may take a while)...");

    let merkle_path_refs: Vec<&str> = merkle_path.iter().map(|s| s.as_str()).collect();
    let (proof_bytes, pub_inputs) = match prove_membership_nullifier(
        data_dir,
        vk_version,
        root_hex,
        scope,
        nullifier_hex,
        leaf_hex,
        secret_hex,
        &merkle_path_refs,
        directions,
    ) {
        Ok(ok) => ok,
        Err(e) => {
            eprintln!("❌ Échec génération preuve: {}", e);
            eprintln!("Astuce: vérifiez que le fichier Poseidon params existe et est synchronisé: {}/zkp/poseidon_params.bin", data_dir);
            eprintln!("Vous pouvez générer les paramètres via: wallet-cli zkp-gen-poseidon-params --data-dir {} (feature zkp_groth16)", data_dir);
            return Err(e);
        }
    };

    // Serialize proof bytes to a binary file alongside the JSON
    let mut bin_path = out.clone();
    bin_path.set_extension("bin");
    let mut bin_f = File::create(&bin_path)?;
    bin_f.write_all(&proof_bytes)?;

    // Prepare JSON envelope
    use ark_ff::{PrimeField, BigInteger};
    let public_inputs_hex: Vec<String> = pub_inputs.iter().map(|fr| {
        let bytes = fr.into_bigint().to_bytes_be();
        hex::encode(bytes)
    }).collect();

    let envelope = serde_json::json!({
        "scheme": "Groth16",
        "vk_version": vk_version,
        "root": root_hex,
        "scope": scope,
        "nullifier": nullifier_hex,
        "proof_file": bin_path.to_string_lossy(),
        "public_inputs": public_inputs_hex,
    });

    let mut f = File::create(out)?;
    f.write_all(serde_json::to_string_pretty(&envelope)?.as_bytes())?;

    println!("Proof written to {} and {}", out.display(), bin_path.display());
    Ok(())
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
    key_id: &Option<String>,
    passphrase: &Option<String>,
    store: &Option<String>,
    node_url: &str,
    json: bool,
) -> Result<()> {
    use crypto_lib::KeyPair;
    use common::{Transaction, TransactionType, proposal::Proposal};
    use uuid::Uuid;
    use chrono::Utc;
    
    info!("Création d'une proposition de loi...");

    // 1. Charger la clé privée (keystore si key_id fourni, sinon fichier)
    let secret = resolve_secret(key_file, key_id, passphrase, store)?;
    let keypair = KeyPair::from_private_bytes(&secret);

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
        if json {
            let out = serde_json::json!({
                "tx_id": tx.id.to_string(),
                "status": "ok",
                "node_response": result,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("✅ Transaction envoyée avec succès !");
            println!("📋 Réponse du nœud : {}", serde_json::to_string_pretty(&result)?);
        }
    } else {
        let error_text = response.text().await?;
        error!("❌ Erreur lors de l'envoi : {}", error_text);
        return Err(anyhow::anyhow!("Échec de l'envoi de la transaction"));
    }

    Ok(())
}

async fn did_generate(key_file: &PathBuf, key_id: &Option<String>, passphrase: &Option<String>, store: &Option<String>, json: bool) -> Result<()> {
    use crypto_lib::KeyPair;
    use base64ct::{Base64UrlUnpadded, Encoding};

    let priv_bytes = resolve_secret(key_file, key_id, passphrase, store)?;
    let kp = KeyPair::from_private_bytes(&priv_bytes);
    let pk = kp.public_key().to_bytes();

    // did:key derivation (multicodec 0xED 0x01 + base58btc prefixed z)
    let mut data = Vec::with_capacity(34); data.push(0xED); data.push(0x01); data.extend_from_slice(&pk);
    let did = format!("did:key:z{}", bs58::encode(data).into_string());
    if json {
        let out = serde_json::json!({"did": did, "jwk_x": Base64UrlUnpadded::encode_string(&pk)});
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("did:key = {}", did);
        // Also print JWK x for convenience
        println!("jwk.x = {}", Base64UrlUnpadded::encode_string(&pk));
    }
    Ok(())
}

// Identity-specific helper functions moved to identity.rs module.

async fn identity_root(node_url: &str) -> Result<()> {
    let url = format!("{}/identity/root", node_url.trim_end_matches('/'));
    let res = reqwest::get(&url).await?;
    if !res.status().is_success() {
        anyhow::bail!("{}: {}", res.status(), res.text().await?);
    }
    let v: serde_json::Value = res.json().await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

async fn anonymous_support_mock(proposal_id: &str, key_file: &PathBuf, key_id: &Option<String>, passphrase: &Option<String>, store: &Option<String>, node_url: &str) -> Result<()> {
    use crypto_lib::KeyPair;
    use common::{Transaction, TransactionType};
    use common::identity::zkp_prelude::{AnonymousActionPayload, ProofEnvelope, ProofScheme, compute_scoped_nullifier_hex};

    // Load private key and derive a stable 32-byte secret
    let secret = resolve_secret(key_file, key_id, passphrase, store)?;
    let kp = KeyPair::from_private_bytes(&secret);

    // Fetch current root
    let root_resp = reqwest::get(format!("{}/identity/root", node_url.trim_end_matches('/'))).await?;
    if !root_resp.status().is_success() { anyhow::bail!("root fetch failed: {}", root_resp.status()); }
    let root_json: serde_json::Value = root_resp.json().await?;
    let root_hex = root_json.get("root").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("root missing"))?.to_string();

    // Build scope and nullifier
    let scope = format!("support:{}", proposal_id);
    let nullifier_hex = compute_scoped_nullifier_hex(&scope, &secret);

    // Build proof envelope (mock: scheme Groth16, vk_version 42, proof bytes "OK" for mock verifier)
    let env = ProofEnvelope {
        scheme: ProofScheme::Groth16,
        vk_version: 42,
        root_hex: root_hex,
        scope: scope.clone(),
        nullifier_hex: nullifier_hex,
        proof: b"OK".to_vec(),
        public_inputs: vec![],
    };
    let payload = AnonymousActionPayload { proof_envelope: env, payload: None };

    // Build and sign transaction
    let mut tx = Transaction::new(
        TransactionType::AnonymousSupport { proposal_id: uuid::Uuid::parse_str(proposal_id)?, proof: payload },
        kp.public_key().clone(),
        kp.sign(b"temp"),
        0,
        0,
    );
    let sign_msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
    tx.signature = kp.sign(sign_msg.as_bytes());

    // Submit via RPC
    let rpc_url = format!("{}/rpc/broadcast_transaction", node_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client.post(&rpc_url).json(&serde_json::json!({"transaction": tx})).send().await?;
    if response.status().is_success() {
        let v: serde_json::Value = response.json().await?;
        println!("{}", serde_json::to_string_pretty(&v)?);
        Ok(())
    } else {
        anyhow::bail!("{}: {}", response.status(), response.text().await?)
    }
}

async fn anonymous_vote_mock(law_id: &str, key_file: &PathBuf, key_id: &Option<String>, passphrase: &Option<String>, store: &Option<String>, node_url: &str) -> Result<()> {
    use crypto_lib::KeyPair;
    use common::{Transaction, TransactionType};
    use common::identity::zkp_prelude::{AnonymousActionPayload, ProofEnvelope, ProofScheme, compute_scoped_nullifier_hex};

    // Load private key and derive a stable 32-byte secret
    let secret = resolve_secret(key_file, key_id, passphrase, store)?;
    let kp = KeyPair::from_private_bytes(&secret);

    // Fetch current root
    let root_resp = reqwest::get(format!("{}/identity/root", node_url.trim_end_matches('/'))).await?;
    if !root_resp.status().is_success() { anyhow::bail!("root fetch failed: {}", root_resp.status()); }
    let root_json: serde_json::Value = root_resp.json().await?;
    let root_hex = root_json.get("root").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("root missing"))?.to_string();

    // Build scope and nullifier
    let scope = format!("vote:{}", law_id);
    let nullifier_hex = compute_scoped_nullifier_hex(&scope, &secret);

    // Build proof envelope (mock)
    let env = ProofEnvelope {
        scheme: ProofScheme::Groth16,
        vk_version: 42,
        root_hex: root_hex,
        scope: scope.clone(),
        nullifier_hex: nullifier_hex,
        proof: b"OK".to_vec(),
        public_inputs: vec![],
    };
    let payload = AnonymousActionPayload { proof_envelope: env, payload: None };

    // Build and sign transaction
    let mut tx = Transaction::new(
        TransactionType::AnonymousVote { law_id: uuid::Uuid::parse_str(law_id)?, proof: payload },
        kp.public_key().clone(),
        kp.sign(b"temp"),
        0,
        0,
    );
    let sign_msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
    tx.signature = kp.sign(sign_msg.as_bytes());

    // Submit via RPC
    let rpc_url = format!("{}/rpc/broadcast_transaction", node_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client.post(&rpc_url).json(&serde_json::json!({"transaction": tx})).send().await?;
    if response.status().is_success() {
        let v: serde_json::Value = response.json().await?;
        println!("{}", serde_json::to_string_pretty(&v)?);
        Ok(())
    } else {
        anyhow::bail!("{}: {}", response.status(), response.text().await?)
    }
}

#[cfg(feature = "zkp_groth16")]
async fn zkp_setup_keys(data_dir: &str, vk_version: u32) -> Result<()> {
    use crate::zkp::setup_placeholder_keys;
    setup_placeholder_keys(data_dir, vk_version)?;
    println!("✅ Clés VK/PK générées dans {}/zkp pour vk_version={}", data_dir, vk_version);
    Ok(())
}

#[cfg(feature = "zkp_groth16")]
async fn anonymous_support_real(proposal_id: &str, key_file: &PathBuf, key_id: &Option<String>, passphrase: &Option<String>, store: &Option<String>, vc_file: &PathBuf, node_url: &str, data_dir: &str, vk_version: u32) -> Result<()> {
    use crypto_lib::KeyPair;
    use common::{Transaction, TransactionType};
    use common::identity::zkp_prelude::{AnonymousActionPayload, ProofEnvelope, ProofScheme, compute_scoped_nullifier_hex};
    use crate::zkp::prove_membership_nullifier;

    // Load private key and derive stable secret
    let secret = resolve_secret(key_file, key_id, passphrase, store)?;
    let kp = KeyPair::from_private_bytes(&secret);

    // Compute commitment hash from VC
    let vc_json = std::fs::read_to_string(vc_file)?;
    let vc_value: serde_json::Value = serde_json::from_str(&vc_json)?;
    let commitment_hash = identity::vc_compute_hash(&vc_value)?;
    let leaf_hex = commitment_hash.clone();

    // Fetch Merkle proof from node
    let proof_url = format!("{}/identity/proof/{}", node_url.trim_end_matches('/'), commitment_hash);
    let proof_resp = reqwest::get(&proof_url).await?;
    if !proof_resp.status().is_success() { anyhow::bail!("proof fetch failed: {}", proof_resp.status()); }
    let proof_json: serde_json::Value = proof_resp.json().await?;
    let path_array = proof_json.get("path").and_then(|v| v.as_array()).ok_or_else(|| anyhow::anyhow!("path missing"))?;
    let mut merkle_path_hex = Vec::new();
    let mut directions = String::new();
    for item in path_array {
        let sib_hex = item.get(0).and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("sib hex missing"))?;
        let is_left = item.get(1).and_then(|v| v.as_bool()).ok_or_else(|| anyhow::anyhow!("is_left missing"))?;
        merkle_path_hex.push(sib_hex.to_string());
        directions.push(if is_left { '0' } else { '1' });
    }

    // Fetch node root (for verification)
    let root_resp = reqwest::get(format!("{}/identity/root", node_url.trim_end_matches('/'))).await?;
    if !root_resp.status().is_success() { anyhow::bail!("root fetch failed: {}", root_resp.status()); }
    let root_json: serde_json::Value = root_resp.json().await?;
    let root_hex = root_json.get("root").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("root missing"))?.to_string();

    // Build scope and nullifier
    let scope = format!("support:{}", proposal_id);
    let nullifier_hex = compute_scoped_nullifier_hex(&scope, &secret);

    // Private inputs
    let secret_hex = hex::encode(secret);

    // Prove
    let (proof_bytes, pub_inputs) = prove_membership_nullifier(data_dir, vk_version, &root_hex, &scope, &nullifier_hex, &leaf_hex, &secret_hex, &merkle_path_hex.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &directions)?;
    // For transparency, include hex of inputs
    let public_inputs_hex = pub_inputs.iter().map(|fr| {
        use ark_ff::{PrimeField, BigInteger};
        let bytes = fr.into_bigint().to_bytes_be();
        hex::encode(bytes)
    }).collect::<Vec<_>>();

    let env = ProofEnvelope {
        scheme: ProofScheme::Groth16,
        vk_version,
        root_hex,
        scope: scope.clone(),
        nullifier_hex,
        proof: proof_bytes,
        public_inputs: public_inputs_hex,
    };
    let payload = AnonymousActionPayload { proof_envelope: env, payload: None };

    // Build tx and submit
    let mut tx = Transaction::new(
        TransactionType::AnonymousSupport { proposal_id: uuid::Uuid::parse_str(proposal_id)?, proof: payload },
        kp.public_key().clone(),
        kp.sign(b"temp"),
        0,
        0,
    );
    let sign_msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
    tx.signature = kp.sign(sign_msg.as_bytes());

    let rpc_url = format!("{}/rpc/broadcast_transaction", node_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client.post(&rpc_url).json(&serde_json::json!({"transaction": tx})).send().await?;
    if response.status().is_success() {
        let v: serde_json::Value = response.json().await?;
        println!("{}", serde_json::to_string_pretty(&v)?);
        Ok(())
    } else {
        anyhow::bail!("{}: {}", response.status(), response.text().await?)
    }
}

#[cfg(feature = "zkp_groth16")]
async fn anonymous_vote_real(law_id: &str, key_file: &PathBuf, key_id: &Option<String>, passphrase: &Option<String>, store: &Option<String>, vc_file: &PathBuf, node_url: &str, data_dir: &str, vk_version: u32) -> Result<()> {
    use crypto_lib::KeyPair;
    use common::{Transaction, TransactionType};
    use common::identity::zkp_prelude::{AnonymousActionPayload, ProofEnvelope, ProofScheme, compute_scoped_nullifier_hex};
    use crate::zkp::prove_membership_nullifier;

    let secret = resolve_secret(key_file, key_id, passphrase, store)?;
    let kp = KeyPair::from_private_bytes(&secret);

    // Compute commitment hash from VC
    let vc_json = std::fs::read_to_string(vc_file)?;
    let vc_value: serde_json::Value = serde_json::from_str(&vc_json)?;
    let commitment_hash = identity::vc_compute_hash(&vc_value)?;
    let leaf_hex = commitment_hash.clone();

    // Fetch Merkle proof from node
    let proof_url = format!("{}/identity/proof/{}", node_url.trim_end_matches('/'), commitment_hash);
    let proof_resp = reqwest::get(&proof_url).await?;
    if !proof_resp.status().is_success() { anyhow::bail!("proof fetch failed: {}", proof_resp.status()); }
    let proof_json: serde_json::Value = proof_resp.json().await?;
    let path_array = proof_json.get("path").and_then(|v| v.as_array()).ok_or_else(|| anyhow::anyhow!("path missing"))?;
    let mut merkle_path_hex = Vec::new();
    let mut directions = String::new();
    for item in path_array {
        let sib_hex = item.get(0).and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("sib hex missing"))?;
        let is_left = item.get(1).and_then(|v| v.as_bool()).ok_or_else(|| anyhow::anyhow!("is_left missing"))?;
        merkle_path_hex.push(sib_hex.to_string());
        directions.push(if is_left { '0' } else { '1' });
    }

    // Fetch node root (for verification)
    let root_resp = reqwest::get(format!("{}/identity/root", node_url.trim_end_matches('/'))).await?;
    if !root_resp.status().is_success() { anyhow::bail!("root fetch failed: {}", root_resp.status()); }
    let root_json: serde_json::Value = root_resp.json().await?;
    let root_hex = root_json.get("root").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("root missing"))?.to_string();

    // Build scope and nullifier
    let scope = format!("vote:{}", law_id);
    let nullifier_hex = compute_scoped_nullifier_hex(&scope, &secret);

    // Private inputs
    let secret_hex = hex::encode(secret);

    // Prove
    let (proof_bytes, pub_inputs) = prove_membership_nullifier(data_dir, vk_version, &root_hex, &scope, &nullifier_hex, &leaf_hex, &secret_hex, &merkle_path_hex.iter().map(|s| s.as_str()).collect::<Vec<_>>(), &directions)?;
    let public_inputs_hex = pub_inputs.iter().map(|fr| {
        use ark_ff::{PrimeField, BigInteger};
        let bytes = fr.into_bigint().to_bytes_be();
        hex::encode(bytes)
    }).collect::<Vec<_>>();

    let env = ProofEnvelope {
        scheme: ProofScheme::Groth16,
        vk_version,
        root_hex,
        scope: scope.clone(),
        nullifier_hex,
        proof: proof_bytes,
        public_inputs: public_inputs_hex,
    };
    let payload = AnonymousActionPayload { proof_envelope: env, payload: None };

    let mut tx = Transaction::new(
        TransactionType::AnonymousVote { law_id: uuid::Uuid::parse_str(law_id)?, proof: payload },
        kp.public_key().clone(),
        kp.sign(b"temp"),
        0,
        0,
    );
    let sign_msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
    tx.signature = kp.sign(sign_msg.as_bytes());

    let rpc_url = format!("{}/rpc/broadcast_transaction", node_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client.post(&rpc_url).json(&serde_json::json!({"transaction": tx})).send().await?;
    if response.status().is_success() {
        let v: serde_json::Value = response.json().await?;
        println!("{}", serde_json::to_string_pretty(&v)?);
        Ok(())
    } else {
        anyhow::bail!("{}: {}", response.status(), response.text().await?)
    }
}