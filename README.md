# E-Government Blockchain System

Un système d'e-gouvernement décentralisé permettant la gestion démocratique des lois via blockchain avec authentification forte des citoyens.

## 🎯 Objectifs

- **Transparence**: Toutes les lois et modifications sont publiques et traçables
- **Décentralisation**: Pas de point unique de contrôle
- **Démocratie participative**: Système de vote sécurisé pour les lois
- **Sécurité**: Cryptographie asymétrique, validation d'identité et blockchain
- **Interopérabilité**: Architecture modulaire pour différents modes de déploiement

## 🏗️ Architecture

```
┌─────────────────┐    ┌────────────────────┐    ┌─────────────────────┐
│   Web Interface │    │   API Gateway      │    │   Blockchain Node   │
│     (Yew.rs)    │◄──►│  (Axum + SQLite)   │◄──►│  (Consensus Engine) │
└─────────────────┘    └────────────────────┘    └─────────────────────┘
         │                       │                          │
         │                       │                          │
         ▼                       ▼                          ▼
┌─────────────────┐    ┌────────────────────┐    ┌─────────────────────┐
│  Frontend API   │    │Identity Validation │    │   Storage Layer     │
│  (REST Client)  │    │  (Gov Integration) │    │  (Blocks + Chain)   │
└─────────────────┘    └────────────────────┘    └─────────────────────┘
                                │
                                ▼
                       ┌────────────────────┐
                       │   Crypto Library   │
                       │(Ed25519, AES, SHA) │
                       └────────────────────┘
```

## 🚀 Démarrage rapide

### Prérequis
- Rust 1.70+
- Docker et Docker Compose (optionnel mais recommandé)
- SQLite 3

### Installation et démarrage
```powershell
# Cloner et installer
git clone <repo-url>
cd e-government-blockchain

# Option A: Docker Compose (recommandé)
docker compose up -d --build

# Vérifier la santé
docker compose ps
docker exec egovern-gateway curl -sS http://localhost:8081/health

# Option B: Build local avec Cargo
cargo build --workspace

# Démarrer localement (dev)
# Terminal 1: Serveur blockchain
cargo run -p blockchain-server
# Terminal 2: API Gateway
cargo run -p api-gateway
# Terminal 3: Interface web
cd web-interface; trunk serve
```

## 📁 Structure du projet

- `api-gateway/` - API REST et gestion des utilisateurs
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

- `blockchain-server/` - Serveur blockchain et moteur de consensus
  - `src/` - Code source
    - `api.rs` - API HTTP pour le serveur blockchain
    - `consensus.rs` - Algorithme de consensus
    - `database.rs` - Stockage persistant des blocs
    - `network.rs` - Communication réseau entre nœuds
    - `node.rs` - Gestion d'un nœud blockchain
    - `security.rs` - Validation et sécurité

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

### Mode Standard (docker-compose.yml)
- Architecture complète avec tous les composants
- Base de données SQLite persistante via volume Docker
- API Gateway persistante exposée sur http://localhost:8081 (GET /health)
- DATABASE_URL: sqlite:///app/data/database.db

### Mode Blockchain Uniquement (docker-compose-blockchain-only.yml)
- Déploie uniquement le serveur blockchain
- Idéal pour nœuds de validation décentralisés

### Mode Développement (docker-compose-sqlite.yml)
- Utilise SQLite en mode mémoire
- Parfait pour le développement et les tests rapides

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
- blockchain-server (8080)
- api-gateway (8081)
- web-interface (3000)

API Gateway persistante:
- Binaire: /app/api-gateway-persistent
- Port: 8081 (env PORT)
- DB: sqlite:///app/data/database.db (volume gateway_data:/app/data)
- Health: GET http://localhost:8081/health

Remarque SQLx: les requêtes SQL utilisent sqlx::query en mode runtime avec .bind(...). Pas de macros query!, pas de cargo sqlx prepare requis.

## 🔧 Dépannage

- GLIBC manquante: base image Debian bookworm-slim requise (incluse). Si vous modifiez l’image, installez ca-certificates/openssl.
- DATABASE_URL SQLite: utilisez un chemin conteneur valide et un volume monté sur /app/data.
- Healthcheck 404: assurez-vous que le binaire persistent est lancé et que /health est exposé (c’est le cas dans persistent_gateway).
