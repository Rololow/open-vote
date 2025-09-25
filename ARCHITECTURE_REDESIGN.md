# Refonte d'Architecture E-Government

_Date:_ 2025-09-24  
_Auteur:_ Proposition de réorganisation modulaire (wallet + attestations + blockchain)

## 🎯 Objectif Global
Séparer clairement les responsabilités entre :
- **Wallet Client** (détention des clés privées / signatures locales)
- **Government Server** (attestations d'identité eID signées)
- **API Gateway** (interface publique citoyen & orchestration logique métier)
- **Blockchain Nodes** (immutabilité, consensus, enregistrement des actes civiques)

Tout en garantissant : sécurité cryptographique, traçabilité, extensibilité, migration incrémentale sans rupture.

---
## 🧩 Composants Cibles
| Composant | Rôle principal | Ne doit PAS faire | Clé(s) critiques |
|-----------|----------------|-------------------|------------------|
| Wallet Client | Générer/tenir la clé privée citoyen, signer identités / votes / soutiens | Stocker la clé privée ailleurs que local | Clé privée citoyen (Ed25519) |
| Government Server | Vérifier identité eID & émettre attestation signée | Gérer votes ou propositions | Clé de signature attestation |
| API Gateway | Auth, validation signatures, agrégation propositions/votes | Vérifier documents eID directement | Clé publique gouvernement |
| Blockchain Nodes | Chaîne d'actes (transactions: identité, proposition, soutien, vote, promotion) | Gérer identité hors chaîne | Clés de nœud / P2P |

---
## 🔐 Flux de Confiance & Signatures
**Attestation identité** (credential signé):
```jsonc
{
  "user_pubkey": "...",
  "identity_hash": "sha256(...)",
  "issuance_ts": 1234567890,
  "expiry_ts": 1734567890,
  "version": 1,
  "nonce": "uuid"
}
```
Signature = `Sign(GOV_PRIVATE_KEY, hash(payload_canonique))`  
Vérifiée par l'API Gateway et potentiellement ancrée sur la blockchain (hash).

Tous les votes / soutiens / propositions sont signés localement par le wallet (jamais la passerelle).

---
## 🗂️ Réorganisation des Crates
Nouveaux crates :
- `government-server/`
  - `attestation.rs`, `identity_sources/` (`france_connect.rs`, `cni.rs`), `api.rs`, `config.rs`.
- `wallet-cli/` (ou `wallet-client/`)
  - `commands/`, `keystore.rs`, `attestation_flow.rs`, `signing.rs`, `config.rs`.

Refactors :
- `api-gateway/` : retirer validation documentaire directe → client HTTP vers government-server. Ajouter `attestations.rs`.
- `common/` : ajouter `api_types.rs` (types partagés : `AttestationPayload`, `SignedVote`, `ProposalRequest`, etc.).

---
## 🗃️ Modèle de Données (API Gateway)
```sql
users(
  id UUID PK,
  public_key BLOB UNIQUE,
  created_at TIMESTAMP,
  status TEXT,                 -- pending|validated|revoked
  identity_hash TEXT NULL,
  validated_at TIMESTAMP NULL
);

identity_attestations(
  id INTEGER PK,
  user_id UUID FK,
  attestation_hash TEXT UNIQUE,
  raw_attestation JSON,
  issued_at TIMESTAMP,
  expires_at TIMESTAMP NULL,
  version INTEGER
);

proposals(
  id INTEGER PK,
  proposer_user_id UUID FK,
  title TEXT,
  body TEXT,
  created_at TIMESTAMP,
  status TEXT                  -- open|promoted|rejected
);

supports(
  id INTEGER PK,
  proposal_id INTEGER FK,
  supporter_user_id UUID FK,
  signature BLOB,
  created_at TIMESTAMP
);

votes(
  id INTEGER PK,
  law_id INTEGER FK,
  voter_user_id UUID FK,
  signature BLOB,
  choice TEXT,                 -- yes|no|abstain
  created_at TIMESTAMP
);

laws(
  id INTEGER PK,
  proposal_id INTEGER FK,
  law_ref_hash TEXT,
  enacted_at TIMESTAMP
);
```

---
## 🔄 Séquence Fonctionnelle (Happy Path)
1. `wallet register` → POST `/api/users/register` { public_key } → retourne `user_id`.
2. `wallet identity request` → Government Server `/api/identity/verify` (mock / réel) → attestation.
3. `wallet identity publish` → POST Gateway `/api/identity/attest` → status devient `validated`.
4. `wallet proposal create` → signe + POST `/api/proposals`.
5. `wallet support --proposal <id>` → signe + POST `/api/proposals/{id}/support`.
6. Gateway détecte seuil de soutiens → envoie transaction `PromoteProposal` à blockchain.
7. Blockchain confirme → loi disponible via `/api/laws`.

---
## 🛠️ Plan de Migration Incrémental
| Phase | Objectif | Risque | Sortie attendue |
|-------|----------|--------|-----------------|
| 0 | Stabiliser build actuel (renommages, `database.rs`) | Compilation cassée | `cargo check` vert |
| 1 | Types partagés (attestation, signatures) | Faible | `common/api_types.rs` |
| 2 | Scaffold `government-server` (mock) | Faible | Endpoint `/api/attestations` |
| 3 | Intégrer attestation dans gateway (`identity_attestations`) | Moyen | Table + endpoint `/api/identity/attest` |
| 4 | Wallet CLI MVP (clés locales) | Moyen | Commandes register / attestation / proposal |
| 5 | Transactions blockchain enrichies (identity, proposal, support, promotion) | Moyen | Nouveaux types de blocs |
| 6 | Retrait stockage clé privée serveur + durcissement | Moyen | Aucune clé privée en DB |
| 7 | Retrait web-interface + docs finales | Faible | README & docs à jour |

Détails des Phases :
- **Phase 0**: Corriger `api-gateway/src/database.rs`, finaliser renommages `Persistent*`.
- **Phase 1**: Introduire `AttestationPayload`, `SignedAttestation`, `SignedProposal`, `SignedSupport`, `SignedVote`.
- **Phase 2**: Nouveau crate `government-server` avec clé éphémère (fichier persistable plus tard) + signature attestation simulée.
- **Phase 3**: Migration DB + vérification signature (clé publique gov via env `GOV_ATTEST_PUBKEY_PATH`).
- **Phase 4**: `wallet-cli` (clap): `register`, `identity request`, `identity publish`, `proposal create`, `support`.
- **Phase 5**: Ajout transactions blockchain: `IdentityValidated`, `ProposalCreated`, `SupportAdded`, `LawPromoted`.
- **Phase 6**: Nettoyage endpoints génération clé privée serveur; attestation rotation & révocation.
- **Phase 7**: Suppression `web-interface/`, mise à jour docs & tests E2E.

---
## 🛡️ Considérations Sécurité
- Attestations versionnées (`version` + structure canonique JSON triée avant hash).
- Anti-replay : `nonce` unique + enregistrement `attestation_hash`.
- Double identité : blocage si `identity_hash` déjà validé → procédure de contestation.
- Throttling attestations par `user_id` + IP.
- Government Server isolé réseau (pas accessible public).
- Hash identité dérivé de champs normalisés (nom, prénom, date naissance, pays, type doc) + pepper (serveur gov) → réduit corrélations.

---
## 🔄 Transition Progressive
| Étape | Ancien flux supporté ? | Nouveau actif ? |
|-------|------------------------|-----------------|
| 0 | Oui | Non |
| 2 | Oui (fallback) | Attestation mock |
| 3 | Partiel | Attestation signée |
| 4 | Ancienne webapp | Wallet CLI |
| 6 | Non (clé privée serveur) | Clé client obligatoire |
| 7 | Non | Architecture finale |

---
## 🧪 Tests Clés
- Vérif signature attestation (clé alt → rejet).
- Idempotence publication (même attestation → pas de duplication).
- Conflit identité (même hash / utilisateur différent → refus).
- E2E: register → attestation → proposal → support → promotion loi.
- Re-soumission invalide (attestation expirée).
- Corruption fichier keystore local → message explicite.

---
## ⚙️ Variables d'Environnement (Prévision)
| Variable | Composant | Description |
|----------|-----------|-------------|
| GOV_ATTEST_PRIVKEY_PATH | government-server | Chemin clé privée attestation |
| GOV_ATTEST_PUBKEY_PATH | api-gateway/blockchain | Clé publique attestation |
| ATTESTATION_EXPIRY_DAYS | government-server | Durée validité |
| SUPPORT_PROMOTION_THRESHOLD | api-gateway | Seuil promotion proposition |
| BLOCKCHAIN_ENDPOINT | api-gateway | URL nœud blockchain |
| WALLET_CONFIG_DIR | wallet-cli | Répertoire config / keystore |
| PEER_NODES | blockchain-server | Découverte P2P |

---
## 📌 Modifications Code Attendues
- Ajout crates: `government-server`, `wallet-cli` → update `[workspace.members]`.
- Migration SQL : `identity_attestations`.
- Nouveau fichier `common/src/api_types.rs`.
- Gateways: endpoints `/api/identity/attest`, `/api/proposals`, `/api/proposals/{id}/support` revisités (signature requise).
- Blockchain: nouveaux enums de transaction.

---
## 🛠️ Keystore Wallet (Concept)
- Fichier : `~/.e-gov-wallet/keys.enc` (JSON chiffré AES-GCM).
- Dérivation clé: Argon2id (passphrase utilisateur + salt).
- Structure interne : `{ "public_key": "...", "encrypted_private_key": "...", "nonce": "...", "kdf": { "salt": "...", "params": {...} } }`.
- Backup : commande `wallet backup --outfile backup.json` (chiffré). Option future phrase mnémotechnique.

---
## ✅ Prochaines Actions Immédiates
1. Stabiliser build (`api-gateway` : réparer `database.rs`, finaliser renommages).  
2. Créer types communs d'attestation (`common/api_types.rs`).  
3. Scaffold `government-server` (clé générée au démarrage + endpoint mock).  
4. Ajouter table `identity_attestations` + endpoint stub `/api/identity/attest`.  
5. Scaffold `wallet-cli` (clap + commande `register`).

---
## 📎 Critères de Succès (MVP Wallet + Attestation)
- Aucune clé privée côté serveur pour comptes nouveaux.
- Attestation signée vérifiée avant `status=validated`.
- Création proposition/vote nécessite signature valide.
- Promotion automatique fonctionne via seuil configuré.

---
## 🚀 Post-MVP
- Rotation attestation / renouvellement.
- GUI (egui / Tauri) facultative.
- Mode offline : préparation votes signés hors ligne.
- Backup distant E2E chiffré (optionnel).

---
## 📝 Notes
Cette refonte est conçue pour être incrémentale : chaque phase livre une valeur sans bloquer les suivantes. La priorité initiale reste d'assainir l'état du workspace avant d'ajouter les nouveaux services.

---
# Pistes de Refonte de l'Architecture pour une Décentralisation et une Sécurité Accrues

Ce document propose des modifications majeures pour répondre aux limitations critiques de l'architecture actuelle, notamment la centralisation de la logique métier dans l'API Gateway et les risques pour la vie privée des citoyens.

L'objectif est de faire évoluer le projet d'un **système centralisé utilisant une blockchain** vers un **protocole de gouvernance véritablement décentralisé**.

---

## Axe 1 : Décentralisation de la Logique Applicative et du Point d'Accès

La dépendance à l'API Gateway comme point d'entrée unique et de contrôle est la plus grande faiblesse du système.

### Modification 1.1 : Interaction Directe du Wallet avec la Blockchain

Le Wallet Client doit pouvoir soumettre des transactions (propositions, soutiens, votes) directement à un nœud du réseau P2P, sans passer par l'API Gateway.

**Implémentation :**
1.  **Exposer une API RPC/REST sur les Nœuds Blockchain :** Chaque nœud (`blockchain-server`) doit exposer des points d'entrée sécurisés pour recevoir des transactions signées de la part des clients.
2.  **Mettre à jour le Wallet Client :** Le client doit être capable de :
    *   Découvrir des nœuds actifs sur le réseau (via une liste de bootstrap ou un service de découverte décentralisé).
    *   Forger, signer et envoyer des transactions directement à un ou plusieurs nœuds.

**Nouveau Flux :**
```
Wallet Client -> Signe la transaction -> Envoie à Nœud Blockchain #1 -> Nœud #1 la propage au réseau P2P
```

### Modification 1.2 : Transformer l'API Gateway en un "Indexer" Optionnel

L'API Gateway perd son rôle de contrôle et devient un service de consultation et d'agrégation de données, similaire à un explorateur de blocs.

**Nouveau Rôle :**
*   **Lire la blockchain :** Indexer les lois, propositions, et états pour fournir une vue agrégée et rapide aux applications clientes.
*   **Fournir des services non critiques :** Notifications, statistiques, historique utilisateur.
*   **Faciliter l'accès :** Les clients légers (mobiles, web) pourraient l'utiliser pour ne pas avoir à interagir directement avec un nœud lourd.

L'utilisation de l'API Gateway devient une commodité, non une nécessité.

### Modification 1.3 : Déplacer la Logique de Promotion des Lois vers le Protocole

La règle "une proposition devient une loi après X soutiens" doit être appliquée par le réseau lui-même, pas par un service central.

**Implémentation :**
*   **Logique de Consensus Enrichie ou Smart Contracts :**
    *   Les nœuds de la blockchain doivent vérifier automatiquement si une proposition a atteint le seuil de soutien requis.
    *   Lorsqu'un bloc est miné, le protocole peut vérifier l'état des propositions et, si un seuil est atteint, générer automatiquement une transaction `LawPromoted` incluse dans le bloc suivant.
    *   Cette logique devient une règle immuable du protocole, auditable par tous.

---

## Axe 2 : Renforcement Radical de la Vie Privée avec la Preuve à Connaissance Nulle (ZKP)

Le lien pseudonyme entre une identité et ses actions sur la chaîne reste un risque majeur pour la vie privée.

### Modification 2.1 : Anonymisation des Actions Citoyennes

**Objectif :** Permettre à un citoyen de prouver qu'il a le droit de voter/soutenir une loi, sans révéler publiquement qui il est ni quelle action il effectue.

**Implémentation (Exemple avec le vote) :**
1.  **Engagement d'Identité :** Lors de la validation par le `Government Server`, l'utilisateur ne publie pas son attestation directement. Il publie un "engagement" cryptographique (un hash de son identité + un secret) sur la chaîne.
2.  **Génération de Preuve ZKP :** Pour voter, le Wallet Client génère une preuve à connaissance nulle qui démontre :
    *   "Je suis bien l'une des personnes dont l'engagement est sur la chaîne."
    *   "Je n'ai pas encore voté pour cette proposition."
3.  **Transaction de Vote Anonyme :** La transaction envoyée à la blockchain ne contient que la preuve et le choix de vote (Pour/Contre). Elle est totalement décorrélée de l'identité publique de l'utilisateur.

**Avantages :**
*   **Secret du vote total :** Impossible de lier un vote à une personne.
*   **Prévention de la coercition :** Personne ne peut prouver comment quelqu'un d'autre a voté.
*   **Maintien de l'intégrité :** Le système garantit toujours "une personne, un vote".

---

## Axe 3 : Décentralisation de la Source de Confiance

La dépendance à un unique `Government Server` comme source de vérité pour l'identité des citoyens est un point de défaillance unique (Single Point of Failure). Pour y remédier, il faut adopter un modèle de "réseau de confiance" basé sur les standards de l'identité décentralisée (W3C).

### Modification 3.1 : Adopter les Identifiants Décentralisés (DID) et les Crédentiels Vérifiables (VC)

L'idée est de passer d'une attestation propriétaire à un système interopérable où plusieurs entités peuvent émettre des preuves d'identité.

*   **Identifiants Décentralisés (DID)** : Chaque participant (citoyen, organisation gouvernementale) possède son propre identifiant cryptographique qu'il contrôle. C'est l'équivalent d'une URL pour une identité.
*   **Crédentiels Vérifiables (VC)** : Ce sont des attestations numériques signées par un **Émetteur** (une autorité de confiance) et détenues par l'utilisateur dans son wallet. Un VC prouve une information (ex: "est un citoyen de plus de 18 ans").

### Nouveau Flux de Validation d'Identité

1.  **Registre des Émetteurs sur la Blockchain** :
    *   La blockchain maintient une liste publique des **Émetteurs** de confiance (leurs DIDs et clés publiques).
    *   La gouvernance du protocole peut décider d'ajouter ou de retirer des émetteurs (ex: agences nationales, mairies, etc.).

2.  **Obtention d'un Crédentiel par le Citoyen** :
    *   Le citoyen, via son Wallet, demande un VC à un Émetteur de son choix.
    *   Après authentification (par des moyens classiques), l'Émetteur signe un VC contenant les "claims" nécessaires (ex: `type: PreuveDeCitoyenneté`) et le renvoie au citoyen.
    *   Le citoyen est le seul détenteur de ce VC dans son Wallet.

3.  **Utilisation et Vérification Anonyme** :
    *   Pour une action (vote, proposition), le Wallet génère une **Preuve à Connaissance Nulle (ZKP)** à partir du VC.
    *   Cette preuve est envoyée à la blockchain. Elle prouve que l'utilisateur détient un VC valide d'un émetteur de confiance, sans révéler le VC lui-même ni l'identité de l'utilisateur.
    *   Les nœuds du réseau vérifient la preuve ZKP et la validité de l'émetteur (en consultant le registre sur la chaîne) avant d'accepter la transaction.

### Avantages

*   **Pas de Point de Défaillance Unique** : La panne ou la corruption d'un émetteur n'arrête pas le système.
*   **Contrôle Utilisateur (Self-Sovereign Identity)** : L'utilisateur gère ses propres preuves d'identité.
*   **Interopérabilité** : Le système devient compatible avec un écosystème mondial d'identité numérique.

---

## Nouvelle Architecture Cible (Schéma)

```
┌──────────────────────────────────────────────────────────────────────────┐
│                          Utilisateur / Citoyen                           │
│                (Wallet Client avec capacités DID, VC & ZKP)              │
└──────────────────────────────────────────────────────────────────────────┘
       │ (1. Demande de VC)                           ▲
       │                                              │ (5. Lecture de la chaîne)
       ▼                                              │
┌─────────────────────────┐                       ┌──────────────────────────┐
│  Réseau d'Émetteurs     │                       │   API Gateway (Indexer)  │
│  (Agences Gov., Mairies)│                       │   (Optionnel, lecture    │
│  - Emettent des VCs     │                       │    seule)                │
└─────────────────────────┘                       └─────────────▲──────────┘
       │ (2. Le Wallet reçoit et stocke le VC)        │
       │ (3. Le Wallet publie un engagement d'identité si nécessaire) │
       │ (4. Le Wallet envoie des transactions anonymes avec ZKP)    │
       ▼                                                             │
┌──────────────────────────────────────────────────────────────────────────┐
│                        Réseau Blockchain P2P                             │
│   ┌────────────────┐    ┌────────────────┐    ┌────────────────┐        │
│   │  Nœud #1       │    │  Nœud #2       │ .. │  Nœud #N       │        │
│   │- Registre DID  │    │- API RPC       │    │- Logique de    │        │
│   │- Validation ZKP│    │- Propagation   │    │  promotion     │        │
│   └────────────────┘    └────────────────┘    └────────────────┘        │
│   Transactions: AnonymousVote | AnonymousProposal | IssuerRegistered...    │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Plan d'Implémentation Incrémental

Adopter cette nouvelle architecture est un projet majeur. Il est crucial de **ne pas recommencer de zéro**, mais de procéder par une refonte incrémentale en s'appuyant sur les bases solides du projet actuel (`crypto-lib`, `common`, `blockchain-server`)

Voici un plan d'action concret en trois phases.

### Phase 1 : Décentralisation du Point d'Accès (Gain le plus rapide)

**Objectif :** Permettre à un client d'interagir directement avec la blockchain, rendant l'API Gateway optionnelle.

1.  **Modifier `blockchain-server` :**
    *   Ajouter un module `rpc_api.rs` pour exposer une API REST/RPC sécurisée.
    *   Créer un premier endpoint `/broadcast_transaction` qui accepte une transaction signée, la vérifie (en utilisant `crypto-lib`), et la propage au réseau P2P.

2.  **Créer un `wallet-cli` de Test :**
    *   Ajouter un nouveau *crate* au workspace (`wallet-cli`).
    *   Ce client en ligne de commande devra :
        *   Générer une paire de clés (`crypto-lib`).
        *   Forger une transaction (`common`).
        *   Signer la transaction (`crypto-lib`).
        *   Envoyer la transaction au endpoint `/broadcast_transaction` d'un nœud.

3.  **Refondre `api-gateway` :**
    *   Commencer à déprécier et retirer les endpoints qui modifient l'état (création de propositions, etc.).
    *   Transformer progressivement l'API Gateway en un simple **lecteur/indexer** de la base de données de la blockchain, fournissant des vues agrégées pour les clients légers.

**Résultat de la Phase 1 :** Le système fonctionne de manière décentralisée pour les transactions de base. L'API Gateway n'est plus un point de contrôle critique.

### Phase 2 : Intégration des Standards d'Identité (DID/VC)

**Objectif :** Remplacer le système d'attestation propriétaire par des standards interopérables, préparant le terrain pour la décentralisation de la confiance.

1.  **Adapter les Structures de `common` :**
    *   Intégrer des bibliothèques Rust pour les DIDs et VCs (ex: `ssi`).
    *   Modifier les structures de transaction pour qu'elles puissent embarquer un VC ou une référence à un VC.

2.  **Faire évoluer le `Government Server` :**
    *   Le transformer en un **Émetteur de VC**. Au lieu de retourner une attestation propriétaire, il générera et signera un Crédentiel Vérifiable standard.

3.  **Mettre à jour la Logique de Validation dans `blockchain-server` :**
    *   La logique de validation des transactions devra maintenant vérifier la signature et la validité du VC attaché, en s'assurant que l'émetteur est (pour l'instant) l'unique `Government Server` de confiance.

**Résultat de la Phase 2 :** Le système utilise des standards d'identité ouverts, le rendant plus robuste et interopérable.

### Phase 3 : Anonymisation via Preuves à Connaissance Nulle (ZKP)

**Objectif :** Garantir le secret des actions citoyennes (votes, soutiens). C'est la phase la plus complexe.

1.  **Choisir une Bibliothèque ZKP :**
    *   Sélectionner une bibliothèque Rust éprouvée et auditée (ex: `arkworks`, `bellman`)

2.  **Définir et Implémenter un Circuit ZKP Simple :**
    *   Commencer avec un cas d'usage unique, comme le vote. Le circuit devra prouver : "Je possède un VC de citoyenneté valide ET je n'ai pas encore voté pour cette loi".

3.  **Intégrer la Génération et la Vérification de Preuves :**
    *   **Côté `wallet-cli` :** Ajouter la capacité de générer la preuve ZKP.
    *   **Côté `blockchain-server` :** Ajouter la capacité de vérifier cette preuve lors de la validation de la transaction. La transaction de vote ne contiendra plus que la preuve et le choix, la rendant anonyme.

**Résultat de la Phase 3 :** Le système garantit un anonymat fort pour les actions les plus sensibles, atteignant le plus haut niveau de sécurité et de respect de la vie privée envisagé par la nouvelle architecture.
