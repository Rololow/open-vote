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
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                     API Modulaire                                   │  │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │  │
│  │  │ General     │ │ Blockchain  │ │ Accounts    │ │ P2P         │   │  │
│  │  │ Handlers    │ │ Handlers    │ │ Handlers    │ │ Handlers    │   │  │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │  │
│  │  ┌─────────────┐ ┌─────────────┐                                   │  │
│  │  │ RPC         │ │ Migrated    │     Types & Routes                │  │
│  │  │ Handlers    │ │ Handlers    │                                   │  │
│  │  └─────────────┘ └─────────────┘                                   │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│  - Communication RPC directe (POST /rpc/broadcast_transaction)            │
│  - Validation cryptographique intégrée                                   │
│  - Consensus et stockage unifiés                                         │
│  - Endpoints API organisés par domaine fonctionnel                       │
└──────────┬───────────────────────────────────────────────────────────────┘
           │  (propagation P2P automatique) 
           ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                        Réseau Blockchain P2P                             │
│   ┌────────────────┐    ┌────────────────┐    ┌────────────────┐        │
│   │  Nœud #1       │    │  Nœud #2       │ .. │  Nœud #N       │        │
│   │ - Consensus    │    │ - Stockage     │    │ - Diffusion    │        │
│   │ - API Unifiée  │    │ - Propagation  │    │ - Validation   │        │
│   └────────────────┘    └────────────────┘    └────────────────┘        │
│   Transactions: CreateAccount | CreateProposal | SupportProposal |       │
│                 CreateLaw | SubmitVote | (extensible)                     │
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

## 🤝 Contribution

Ce projet est en développement actif. Consultez la roadmap pour les prochaines étapes et les fonctionnalités à venir.

## 📄 Licence

MIT OR Apache-2.0

---

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
