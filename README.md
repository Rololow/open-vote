# E-Government Blockchain System

Un système d'e-gouvernement décentralisé permettant la gestion démocratique des lois via blockchain avec authentification forte des citoyens.

## 🎯 Objectifs

- **Tr## ⚙️ Détails techniques

### Services P2P Multi## 🔧 Dépannage

### Problèmes P2P
- **Pairs non détectés**: Vérifiez que les nœuds sont sur le même réseau Docker
  ```powershell
  docker network ls
  docker network inspect <network_name>
  ```
- **Échec de connexion P2P**: Validez les ports et la configuration PEER_NODES
  ```powershell
  curl http://localhost:8081/api/peers
  curl http://localhost:8082/api/peers
  ```
- **Synchronisation bloquée**: Redémarrez les nœuds pour forcer la re-synchronisation
  ```powershell
  docker compose -f docker-compose-rust-only.yml restart
  ```

### Problèmes généraux
- **GLIBC manquante**: Base image Debian bookworm-slim requise (incluse)
- **DATABASE_URL SQLite**: Utilisez un chemin conteneur valide avec volume monté sur /app/data
- **Healthcheck 404**: Vérifiez que le binaire correct est lancé et que /health est exposé
- **Port déjà utilisé**: Changez les ports dans docker-compose ou arrêtez le processus conflictuels (docker-compose-rust-only.yml)
- **blockchain-rust** (nœud principal)
  - Port: 8081
  - Volume: blockchain_data:/app/data
  - Logs: blockchain_logs:/app/logs
  - P2P: Détection automatique des pairs
  
- **blockchain-rust-node2** (nœud secondaire)
  - Port: 8082
  - Volume: blockchain_data_node2:/app/data 
  - Logs: blockchain_logs_node2:/app/logs
  - P2P: Se connecte au nœud principal

### API Endpoints Blockchain P2P
- `GET /health` - État de santé du nœud
- `GET /api/peers` - Liste des pairs P2P connectés
- `POST /api/peers` - Ajouter un nouveau pair
- `POST /api/p2p/block` - Recevoir un bloc d'un pair
- `POST /api/p2p/tx` - Recevoir une transaction d'un pair
- `POST /api/mine` - Miner un nouveau bloc
- `GET /api/chain` - Obtenir la chaîne complète

### Services Architecture Complète
- **blockchain-server**: Port 3000
- **web-interface**: Port 3001

### Configuration P2P
- **Variables d'environnement**:
  - `PEER_NODES`: Liste des pairs (ex: "http://blockchain-rust:8081")
  - `NODE_ID`: Identifiant unique du nœud
  - `PORT`: Port d'écoute du nœud
- **Découverte**: Les nœuds s'enregistrent mutuellement automatiquement
- **Synchronisation**: Propagation automatique des blocs et transactionsToutes les lois et modifications sont publiques et traçables
- **Décentralisation**: Pas de point unique de contrôle
- **Démocratie participative**: Système de vote sécurisé pour les lois
- **Sécurité**: Cryptographie asymétrique, validation d'identité et blockchain
- **Interopérabilité**: Architecture modulaire pour différents modes de déploiement

## 🏗️ Architecture

La nouvelle architecture sépare clairement la détention des clés (client) de la logique métier (gateway) et de l'autorité d'attestation (serveur gouvernemental), tout en conservant une couche blockchain indépendante pour l'immutabilité et la gouvernance.

### Vue Globale (Architecture Unifiée - Phase 1 Complétée)
```
┌──────────────────────────────────────────────────────────────────────────┐
│                          Utilisateur / Citoyen                           │
└──────────────────────────────────────────────────────────────────────────┘
                 │ (RPC signée)                      ▲
                 │                                   │ (Attestation signée)
                 ▼                                   │
┌─────────────────────────┐         (canal privé)   ┌──────────────────────────┐
│  Wallet CLI ✅          │ <────────────────────── │   Government Server      │
│  - Génère clés Ed25519  │  Requête identité       │ (eID vérification +      │
│  - Signe transactions   │ ──────────────────────► │  signature attestation)  │
│  - Communication RPC    │                         └──────────────────────────┘
│  - Gestion locale clés  │                               │
└──────────┬──────────────┘                               │ (clé publique gov)
           │  (transactions signées via RPC)              │
           ▼                                              ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                    Serveur Blockchain Unifié ✅                          │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │                     API Modulaire                                  │  │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │  │
│  │  │ General     │ │ Blockchain  │ │ Accounts    │ │ P2P         │   │  │
│  │  │ Handlers    │ │ Handlers    │ │ Handlers    │ │ Handlers    │   │  │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │  │
│  │  ┌─────────────┐ ┌─────────────┐                                   │  │
│  │  │ RPC         │ │ Migrated    │     Types & Routes                │  │
│  │  │ Handlers    │ │ Handlers    │                                   │  │
│  │  └─────────────┘ └─────────────┘                                   │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│  - Communication RPC directe (POST /rpc/broadcast_transaction)           │
│  - Validation cryptographique intégrée                                   │
│  - Consensus et stockage unifiés                                         │
│  - Endpoints API organisés par domaine fonctionnel                       │
└──────────┬───────────────────────────────────────────────────────────────┘
           │  (propagation P2P automatique) 
           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                        Réseau Blockchain P2P                             │
│   ┌────────────────┐    ┌────────────────┐    ┌────────────────┐         │
│   │  Nœud #1       │    │  Nœud #2       │ .. │  Nœud #N       │         │
│   │ - Consensus    │    │ - Stockage     │    │ - Diffusion    │         │
│   │ - API Unifiée  │    │ - Propagation  │    │ - Validation   │         │
│   └────────────────┘    └────────────────┘    └────────────────┘         │
│   Transactions: CreateAccount | CreateProposal | SupportProposal |       │
│                 CreateLaw | SubmitVote | (extensible)                    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Responsabilités par Composant (Architecture Unifiée)
- **Wallet CLI ✅** : garde exclusive des clés privées Ed25519, signatures locales des transactions, communication RPC directe avec les nœuds blockchain. Aucun secret ne transite vers le serveur.
- **Government Server** : vérifie (ou simule en mode dev) l'identité via sources eID et émet une attestation signée (credential) contenant un hash d'identité pseudonymisé.
- **Serveur Blockchain Unifié ✅** : 
  - API modulaire avec handlers organisés par domaine fonctionnel
  - Validation cryptographique des transactions signées
  - Consensus distribué et stockage persistant
  - Communication P2P pour la propagation des blocs et transactions
  - Endpoints RPC pour communication directe avec les wallets
  - Intégration complète : validation → consensus → stockage → propagation

### Flux Clé (Architecture Unifiée - Phase 1)
1. Le Wallet CLI génère la clé Ed25519 localement (`wallet-cli generate-keypair`).
2. Il crée une transaction signée (proposition, compte, etc.) (`wallet-cli create-proposal`).
3. Il envoie directement la transaction au nœud blockchain via RPC (`POST /rpc/broadcast_transaction`).
4. Le nœud unifié valide la signature, ajoute à la mempool et propage via P2P.
5. Le consensus distribué traite la transaction et l'inscrit définitivement sur la chaîne.
6. (Futur Phase 2) : Intégration des attestations gouvernementales via DID/VC standards.

### Sécurité & Isolation
- Aucune clé privée citoyenne côté serveur.
- Attestation versionnée + anti‑replay (hash + nonce).
- Government Server peut être isolé réseau (non exposé publiquement), la Gateway ne stocke que la clé publique de vérification.
- La blockchain ne "comprend" que des transactions structurées, sans dépendre des formats d'attestation internes.

### Évolution vs Ancienne Architecture

#### Phase 1 Complétée ✅ (Décentralisation du Point d'Accès)
- **Serveur Unifié** : Fusion réussie des services en un seul `blockchain-server` 
- **API Modulaire** : Restructuration du code monolithique en modules organisés par domaine
- **Wallet CLI** : Client en ligne de commande fonctionnel pour interaction directe
- **Communication RPC** : Protocol de communication directe wallet ↔ blockchain
- **Architecture Décentralisée** : Élimination de la dépendance centralisée

#### Prochaine Phase 2 (DID/VC Standards) 🚧
- Intégration des standards W3C DID (Identités Décentralisées)  
- Support des Verifiable Credentials pour attestations gouvernementales
- Migration vers des standards interopérables d'identité

#### Phase 3 Planifiée (Zero-Knowledge Proofs) 🔮  
- Implémentation ZK-SNARKs/STARKs pour votes anonymes
- Protection de la vie privée tout en maintenant la vérifiabilité

## Phase 3 — Zero‑Knowledge Proofs (ZKP)

This project now includes an end-to-end ZKP design (Phase 3) where the Wallet generates a SNARK proof that proves:
- membership in a Merkle commitment set (Merkle root) and
- a scoped nullifier computed from a per-scope secret (so the same secret cannot be reused across scopes).

High-level notes
- Curve / SNARK: BN254 + Groth16 (arkworks). The implementation uses a Poseidon-based circuit for SNARK‑friendly hashing.
- Persistent artifacts (shared between wallet and node): Poseidon params and Groth16 keys are written to the node's data directory under `zkp/`.

Files and locations
- Poseidon parameters: `<data_dir>/zkp/poseidon_params.bin`
- Verifying key (node expects): `<data_dir>/zkp/vk-groth16-v{version}.bin`
- Proving key (wallet): `<data_dir>/zkp/pk-groth16-v{version}.bin`

Wallet: creating a proof
- The wallet CLI can generate Poseidon parameters, perform Groth16 setup (pk/vk) and produce a proof (binary) together with a small JSON envelope containing the public inputs.
- Public input ordering in the circuit: `[root, nullifier, scope_hash]` — the node verification expects the same order.

Node: verification endpoint
- The blockchain server exposes a verification endpoint that consumes the JSON envelope (or inline proof) and verifies the proof against the persisted verifying key:

- POST /api/identity/verify_zkp
  - JSON body fields (either provide `proof` as hex/base64 or `proof_file` pointing to an uploaded binary):
    - `vk_version`: integer (selects `vk-groth16-v{vk_version}.bin` stored under `<data_dir>/zkp/`)
    - `public_inputs`: array of hex strings (32-byte big-endian hex) in the order `[root, nullifier, scope_hash]`
    - `proof`: hex or base64-encoded proof bytes (optional if `proof_file` is provided)

Example JSON (POST body):

```json
{
  "vk_version": 1,
  "public_inputs": ["<root_hex>", "<nullifier_hex>", "<scope_hex>"],
  "proof": "<proof_hex_or_base64>"
}
```

Quick curl example (replace placeholders):

```powershell
# Example: POST proof as inline hex/base64 JSON
curl -X POST http://localhost:3000/api/identity/verify_zkp \
  -H "Content-Type: application/json" \
  -d '{ "vk_version": 1, "public_inputs": ["<root_hex>", "<null_hex>", "<scope_hex>"], "proof": "<proof_hex>" }'
```

### Operator guide (quick)

For operators and CI: a short automation + diagnostics guide lives in `docs/zkp_run.md`. It shows the exact PowerShell script used for the end-to-end automation, how to persist artifacts via `BLOCKCHAIN_DATA_DIRECTORY`, and which `errors.log` entries to inspect when verification fails.

Quick command (PowerShell):

```powershell
pwsh -File .\scripts\zkp_flow_with_clean_log.ps1
```

See `docs/zkp_run.md` for CI recommendations and troubleshooting tips.

Build / feature notes
- The ZKP code is feature-gated in the crates (feature name used in workspace: `zkp_groth16`). When building locally enable that feature for the `wallet-cli` and `blockchain-server` crates if you want the ZKP code paths compiled.

Troubleshooting
- If verification fails: ensure the wallet and node use the same Poseidon parameters and the same `vk_version` (matching VK/PK pair). The public inputs order must match the circuit's expectation: `[root, nullifier, scope_hash]`.
- The project includes gated diagnostic helpers (compile with feature `zkp_debug`) that synthesize the circuit and print constraint statistics to help debug unsatisfied constraints or public‑input ordering issues.

If you'd like, I can also add a compact example script under `scripts/` demonstrating: (1) setup (generate params & keys), (2) wallet proof generation, and (3) POSTing the proof to the node. Reply with which variant you prefer (PowerShell or POSIX shell) and I'll add it.

L'ancienne interface Web (Yew) reste disponible pour tests mais sera progressivement remplacée par le Wallet CLI et une future interface graphique.


## 🚀 Démarrage rapide

### Prérequis
- Rust 1.70+
- Docker et Docker Compose (optionnel mais recommandé)
- SQLite 3

### Installation et démarrage

#### Mode P2P Multi-Nœuds (Recommandé)
```powershell
# Cloner et installer
git clone <repo-url>
cd e-government-blockchain

# Démarrer le réseau P2P avec 2 nœuds blockchain
docker compose -f docker-compose-rust-only.yml up -d --build

# Vérifier les nœuds
docker compose -f docker-compose-rust-only.yml ps
curl http://localhost:8081/health  # Nœud 1
curl http://localhost:8082/health  # Nœud 2

# Vérifier la découverte P2P
curl http://localhost:8081/api/peers  # Doit montrer le nœud 2 comme pair
curl http://localhost:8082/api/peers  # Doit montrer le nœud 1 comme pair
```

#### Mode Architecture Complète
```powershell
# Interface web + API Gateway + Blockchain
docker compose up -d --build

# Services disponibles:
# - Web Interface: http://localhost:3000
# - API Gateway: http://localhost:8081
# - Blockchain Node: http://localhost:8080
```

#### Mode Développement Local (Architecture Unifiée)
```powershell
# Build du workspace
cargo build --workspace

# Terminal 1: Nœud blockchain principal unifié (Port 3000)
cargo run -p blockchain-server

# Terminal 2: Second nœud (optionnel pour P2P)
RUST_LOG=info PEER_NODES=http://localhost:3000 PORT=3001 cargo run -p blockchain-server

# Terminal 3: Tester avec le Wallet CLI
# Générer des clés
cargo run -p wallet-cli -- generate-keypair --output my-keypair.hex

# Créer une proposition
cargo run -p wallet-cli -- create-proposal \
  --keypair my-keypair.hex \
  --title "Ma première proposition" \
  --description "Description de la proposition" \
  --server http://localhost:3000

# Terminal 4: Interface web (optionnel, legacy)
cd web-interface && trunk serve
```

## 📁 Structure du projet

- `blockchain-server/` - Serveur blockchain unifié avec API REST et gestion des utilisateurs
  - `src/` - Code source
    - `auth.rs` - Authentification et gestion de sessions
    - `crypto_service.rs` - Services cryptographiques
    - `database.rs` - Couche d'accès aux données
    - `identity_validation.rs` - Validation d'identité des citoyens
    - `middleware.rs` - Middleware d'authentification
    - `persistent_database.rs` - Base de données persistante avec SQLite
    - `persistent_gateway.rs` - API gateway persistante
    - `persistent_routes.rs` - Routes API pour le mode persistant
    - `user_management.rs` - Gestion des utilisateurs et identités
  - `migrations/` - Migrations de base de données
  - `tests/` - Tests d'intégration et de performance

- `blockchain-server/` - Serveur blockchain P2P et moteur de consensus
  - `src/` - Code source
    - `api.rs` - API HTTP avec endpoints P2P (peers, block propagation, transaction sharing)
    - `consensus.rs` - Algorithme de consensus distribué
    - `database.rs` - Stockage persistant des blocs avec SQLite
    - `network.rs` - Communication P2P entre nœuds
    - `node.rs` - Gestion d'un nœud blockchain avec découverte de pairs
    - `security.rs` - Validation cryptographique et sécurité blockchain

- `common/` - Types et structures de données partagés
  - `src/`
    - `account.rs` - Gestion des comptes utilisateurs
    - `block.rs` - Structure des blocs
    - `blockchain.rs` - Chaîne de blocs
    - `law.rs` - Structure des lois
    - `transaction.rs` - Transactions et signatures
    - `vote.rs` - Système de vote

- `crypto-lib/` - Bibliothèque cryptographique partagée
  - `src/` - Implémentations cryptographiques
    - `hash.rs` - Fonctions de hachage sécurisées
    - `keys.rs` - Gestion des clés cryptographiques
    - `signatures.rs` - Signatures numériques
  - `tests/` - Tests unitaires

- `web-interface/` - Interface utilisateur en WebAssembly
  - `src/` - Code source frontend
  - `static/` - Ressources statiques

## 🚢 Modes de déploiement

Le système supporte plusieurs configurations de déploiement:

### Mode P2P Multi-Nœuds (docker-compose-rust-only.yml) ⭐ RECOMMANDÉ
- **Architecture**: Réseau P2P avec 2 nœuds blockchain indépendants
- **Ports**: Node 1 (8081), Node 2 (8082) 
- **Stockage**: Volumes persistants séparés par nœud
- **P2P**: Découverte automatique des pairs et synchronisation
- **APIs**: 
  - `/health` - Santé du nœud
  - `/api/peers` - Gestion des pairs P2P
  - `/api/p2p/block` - Propagation de blocs
  - `/api/p2p/tx` - Partage de transactions
  - `/api/mine` - Minage de blocs
- **Utilisation**: Production décentralisée, résilience réseau

### Mode Architecture Complète (docker-compose.yml)
- **Composants**: Web Interface + API Gateway + Blockchain Node
- **Base de données**: SQLite persistante via volume Docker
- **API Gateway**: http://localhost:8081 (authentification, validation d'identité)
- **Web Interface**: http://localhost:3000 (frontend Yew.rs)
- **Utilisation**: Application complète avec interface utilisateur

### Mode Blockchain Uniquement (docker-compose-blockchain-only.yml)
- **Architecture**: Nœud blockchain simple
- **Utilisation**: Nœuds de validation décentralisés, intégration dans réseau existant

### Mode Développement (docker-compose-sqlite.yml)
- **Base de données**: SQLite en mode mémoire
- **Utilisation**: Développement rapide et tests

## 🔐 Sécurité

- **Validation d'identité** des citoyens avec documents officiels
- **Signatures Ed25519** pour les transactions et votes
- **Chiffrement AES-GCM** pour les clés privées
- **Hachage SHA-256** pour l'intégrité des blocs
- **JWT** pour l'authentification des sessions
- **Consensus distribué** pour validation des blocs

## 📖 Documentation

- `PROJECT_ROADMAP.md` - Suivi détaillé du développement
- `BLOCKCHAIN_SECURITY.md` - Mesures de sécurité blockchain
- `DOCKER_DEPLOYMENT.md` - Guide de déploiement avec Docker
- `IMPLEMENTATION_REPORT.md` - Rapport technique d'implémentation
 - `docs/identity_diagram.md` - Diagramme du flux identité (Mermaid)
 - `docs/troubleshooting_identity.md` - Guide de dépannage identité (DID/VC)
 - `docs/examples/vc_citizen.json` - Exemple de VC minimal
 - `docs/merkle_proofs.md` - Preuves de Merkle (racine, génération et vérification de preuves)
 - `docs/merkle_proofs.md` - Merkle root/proofs CLI et API (préparation ZKP)

### Outils CLI (Merkle) 
- `compute_root` (crate `blockchain-server`) calcule la racine Merkle d’un `commitments.log`.
- `commitment_proof` (crate `blockchain-server`) génère et vérifie des preuves pour un index donné.

Voir `docs/merkle_proofs.md` pour les détails de format et l’API, et le paragraphe « Essayer rapidement (preuves Merkle) » dans `ARCHITECTURE_REDESIGN.md` pour des commandes PowerShell prêtes à l’emploi.

## 🤝 Contribution

Ce projet est en développement actif. Consultez la roadmap pour les prochaines étapes et les fonctionnalités à venir.

## 📄 Licence

MIT OR Apache-2.0

# ⚡️ ZKP Poseidon Parameters Synchronization

To ensure successful proof verification, both wallet-cli and blockchain-server must use the exact same Poseidon parameters file.

**Poseidon parameters file location:**
- `<data_dir>/zkp/poseidon_params.bin`

**How to synchronize:**
1. Generate Poseidon parameters once using wallet-cli or server setup.
2. Copy the resulting `poseidon_params.bin` file to both the wallet-cli and server data directories (overwrite any existing file).
3. Confirm the hashes match by running your workflow and checking `errors.log` for:
  - `Poseidon params hash (cli): ...`
  - `Poseidon params hash (serveur): ...`
  - These hashes must be identical for proof verification to succeed.

**Example parameters (BN254, Groth16, recommended for this project):**
- Curve: BN254
- Hash: Poseidon
- Full rounds: 8
- Partial rounds: 57
- Rate: 2
- Capacity: 1
- Alpha: 5
- MDS: 3x3 matrix (see generated file)
- Ark: 65x3 matrix (see generated file)

**Do not modify Poseidon parameters independently on wallet or server. Always use a single, shared file.**

If you regenerate parameters, repeat the copy step to keep both sides in sync.


## ⚙️ Détails d’exécution (compose)

Services:
- blockchain-server (3000) - Serveur blockchain unifié
- web-interface (3001) - Interface web de visualisation
- DB: sqlite:///app/data/database.db (volume gateway_data:/app/data)
- Health: GET http://localhost:8081/health

Remarque SQLx: les requêtes SQL utilisent sqlx::query en mode runtime avec .bind(...). Pas de macros query!, pas de cargo sqlx prepare requis.

## 🔧 Dépannage

- GLIBC manquante: base image Debian bookworm-slim requise (incluse). Si vous modifiez l’image, installez ca-certificates/openssl.
- DATABASE_URL SQLite: utilisez un chemin conteneur valide et un volume monté sur /app/data.
- Healthcheck 404: assurez-vous que le binaire persistent est lancé et que /health est exposé (c’est le cas dans persistent_gateway).

## 🧪 Parcours E2E Identité (DID/VC)

Ce scénario end‑to‑end valide la génération d’une identité locale (did:key), l’émission d’un Verifiable Credential (VC) par l’émetteur intégré au nœud, l’engagement/commitment de l’identité côté blockchain, puis la vérification publique via l’API.

### Exécution rapide (script)

Prerequis: Windows PowerShell, Rust installé. Le script lance le serveur, effectue toute la séquence DID/VC, et valide le résultat.

```powershell
# Depuis la racine du repo
pwsh -NoProfile -File "scripts/e2e_identity.ps1" -ApiPort 8096 -Build:$false
```

Paramètres utiles:
- `-ApiPort`: port HTTP du serveur (par défaut 8085 dans le script)
- `-Build`: force la compilation debug de `blockchain-server` et `wallet-cli` (mettre `$true` la première fois)

Le script affiche les étapes avec des tags [STEP]/[OK] et termine quand la présence du commitment est confirmée côté API.

Tests négatifs optionnels:

```powershell
# Émetteur non autorisé (doit échouer côté commit et rester 404 côté GET)
pwsh -NoProfile -File "scripts/e2e_identity.ps1" -ApiPort 8097 -Build:$false -NegativeMode DisallowedIssuer

# Signature invalide (VC trafiqué, doit échouer commit et rester 404 côté GET)
pwsh -NoProfile -File "scripts/e2e_identity.ps1" -ApiPort 8098 -Build:$false -NegativeMode InvalidSignature
```

### Étapes manuelles (détaillées)

1) Build des binaires (debug ou release)

```powershell
cargo build -p blockchain-server -p wallet-cli
```

2) Lancer le serveur blockchain unifié (avec autorisation d’émetteur large en dev)

```powershell
$env:BLOCKCHAIN_BIND_ADDRESS = "127.0.0.1"
$env:BLOCKCHAIN_API_PORT = "8095"
$env:ALLOWED_ISSUERS_DIDS = ""   # vide => autorise tous les DID émetteurs (dev)
cargo run -p blockchain-server
```

Dans un autre terminal PowerShell, attendre que `GET http://127.0.0.1:8095/health` réponde 200.

3) Générer une paire de clés et un did:key (wallet-cli)

```powershell
# Génère une clé Ed25519 locale
cargo run -p wallet-cli -- generate-keypair --output scripts\wallet_key.hex

# Dérive le DID sujet à partir de la clé
cargo run -p wallet-cli -- did-generate --key-file scripts\wallet_key.hex
# ➜ relevez la valeur did:key: ex: did:key:z6Mkj... (utilisée comme --subject-did)
```

4) Demander un VC à l’émetteur intégré et le sauvegarder localement

```powershell
$outDir = "$env:TEMP\e2e-creds"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
cargo run -p wallet-cli -- vc-request `
  --endpoint http://127.0.0.1:8095 `
  --subject-did <VOTRE_DID_KEY> `
  --out-dir $outDir

# Un fichier <commitment_hash>.json est créé dans $outDir
```

5) Engager/committer le VC côté nœud (auto‑register si manquant), puis vérifier via GET

```powershell
# Le node_url DOIT inclure le préfixe /api
cargo run -p wallet-cli -- vc-commit `
  --file $outDir\<commitment_hash>.json `
  --dir $outDir `
  --node-url http://127.0.0.1:8095/api `
  --issuer-endpoint http://127.0.0.1:8095

# Vérification publique
Invoke-RestMethod -Uri "http://127.0.0.1:8095/api/identity/commitments/<commitment_hash>" -Method GET
```

### Sorties attendues (exemples)

Script e2e (extraits):

```
[STEP] Starting blockchain-server on http://127.0.0.1:8096
[STEP] Waiting for /health…
[OK] /health OK
[STEP] Generating wallet keypair → ...\wallet_key.hex
[STEP] Deriving did:key from ...\wallet_key.hex
[OK] subject DID: did:key:z6Mkj... 
[STEP] Requesting VC from issuer at http://127.0.0.1:8096
[OK] VC saved: C:\Users\...\AppData\Local\Temp\e2e-cred-...\<commitment_hash>.json
[OK] commitment hash: <commitment_hash>
[STEP] Committing VC via wallet-cli (auto-register)
[STEP] Confirming GET /api/identity/commitments/<commitment_hash>
[OK] Commitment present. issuer_did=did:key:zIssuer... did=did:key:zSubject...

E2E identity flow completed successfully.
```

Requête GET JSON attendue:

```json
{
  "commitment_hash": "<commitment_hash>",
  "issuer_did": "did:key:zIssuer...",
  "did": "did:key:zSubject...",
  "id": 1
}
```

### Notes & Dépannage (identité)

- `node_url` doit inclure le préfixe `/api` pour les commandes wallet (`--node-url http://127.0.0.1:8095/api`).
- Contrôle des émetteurs: `ALLOWED_ISSUERS_DIDS` accepte une liste de DID (séparateur virgule). Vide (`""`) = autoriser tous (mode dev).
- Si le GET renvoie 404, laissez le `vc-commit` faire un POST d’auto‑enregistrement (comportement par défaut si la lecture échoue).
- Sous Windows/OneDrive, privilégiez des chemins sans espaces pour éviter des surprises de quoting.

## 🔑 Gestion de la clé d'Issuer (offline)

Un outil en ligne de commande est fourni pour gérer la clé Ed25519 de l'émetteur (issuer) et exporter la JWK publique.

- Binaire: `issuer_key_tool` (dans `blockchain-server`)
- Fichier clé par défaut: `issuer_ed25519_key.json` (modifiable via `ISSUER_KEY_PATH`)
- Variables d'environnement utiles:
  - `ISSUER_KEY_PATH`: chemin du fichier clé issuer (défaut: `issuer_ed25519_key.json`)
  - `ISSUER_VC_VALIDITY_DAYS`: durée de validité des VC émis (jours)
  - `ISSUER_PUBKEY_EXPORT`: si défini (chemin), l'API `/issuer/jwk` peut aussi écrire la JWK publique côté serveur

Exemples d’utilisation:

```powershell
# Créer/charger la clé et afficher le DID émetteur
cargo run -p blockchain-server --bin issuer_key_tool

# Spécifier un chemin de clé et exporter la JWK publique
cargo run -p blockchain-server --bin issuer_key_tool -- `
  --key issuer_ed25519_key.json `
  --export-jwk issuer_pub.jwk

# Importer une clé privée Ed25519 (32 octets hex) puis exporter la JWK
cargo run -p blockchain-server --bin issuer_key_tool -- `
  --key issuer_ed25519_key.json `
  --import-private-hex 00112233...ffeeddcc00112233...ffeeddcc `
  --export-jwk issuer_pub.jwk
```

```bash
# Linux/macOS
cargo run -p blockchain-server --bin issuer_key_tool -- \
  --key issuer_ed25519_key.json \
  --export-jwk issuer_pub.jwk
```

Notes sécurité:
- L’outil n’imprime jamais la clé privée. Conservez `issuer_ed25519_key.json` dans un répertoire protégé.
- La rotation de clé est facilitée par un `kid` déterministe dérivé de la clé publique (préfixe hex). Documentez la co‑existence des anciennes clés si des VC non expirés circulent encore.

## 🌳 Outil Merkle root (commitments)

Un utilitaire CLI est fourni pour calculer la racine de Merkle à partir d'une liste de hachés d'engagements (un hash hexadécimal de 32 octets par ligne).

- Binaire: `compute_root` (inclus dans `blockchain-server`)
- Fichier d'entrée par défaut: `<repo>/blockchain-server/data/commitments.log`
- Sortie: affiche `merkle_root=<hex>` sur la sortie standard; option `--out <path>` pour écrire la racine en hex dans un fichier.

Utilisation:

- `compute_root [<commitments.log>] [--out <path>]`
- Options:
  - `-h, --help`: afficher l'aide et quitter
  - `--out <path>`: écrire la racine calculée dans un fichier en plus de l'afficher

Contraintes d'entrée: chaque ligne non vide doit être un hash hex de 32 octets (64 caractères hex). Si le nombre de feuilles est impair, la dernière est dupliquée pour le calcul du niveau (comportement standard Merkle).

### Format du journal `commitments.log`
- Emplacement par défaut: `blockchain-server/data/commitments.log`
- Format: une ligne par engagement (commitment), chaque ligne contient un hash SHA‑256 encodé en hex (64 caractères).
- Append‑only: de nouvelles lignes sont ajoutées à chaque engagement `active`. Les révocations/expirations sont gérées côté base/état, le log reste historique.
- Outils associés:
  - `compute_root`: calcule une racine de Merkle reproductible
  - `scripts/compute_commitments_root.ps1`: snapshot quotidien (Windows)

## 🧪 CI / Parité Linux

Un script bash minimal est fourni pour CI Linux afin de vérifier le flux identité et lancer les tests.

```bash
./scripts/identity_flow.sh --key issuer_ed25519_key.json --export-jwk issuer_pub.jwk
# Options: --no-cargo pour réutiliser des binaires déjà construits
```

Ce script:
- Construit `issuer_key_tool` (sauf `--no-cargo`)
- S’assure de la présence d’une clé issuer et, si demandé, exporte la JWK publique
- Exécute `cargo test --workspace`
