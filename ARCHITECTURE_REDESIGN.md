# Refonte d'Architecture E-Government

_Date:_ 2025-09-24  
_Auteur:_ Proposition de réorganisation modulaire (wallet + attestations + blockchain)

## 🎯 Objectif Global
Séparer clairement les responsabilités entre :
- **Wallet Client** (détention des clés privées / signatures locales)
- **API Gateway** (interface publique citoyen & orchestration logique métier)
Tout en garantissant : sécurité cryptographique, traçabilité, extensibilité, migration incrémentale sans rupture.

---
## 🧩 Composants Cibles
| Composant | Rôle principal | Ne doit PAS faire | Clé(s) critiques |
| Wallet Client | Générer/tenir la clé privée citoyen, signer identités / votes / soutiens | Stocker la clé privée ailleurs que local | Clé privée citoyen (Ed25519) |
---
**Attestation identité** (credential signé):
  "issuance_ts": 1234567890,
  "version": 1,
  "nonce": "uuid"
}
Signature = `Sign(GOV_PRIVATE_KEY, hash(payload_canonique))`  
Vérifiée par l'API Gateway et potentiellement ancrée sur la blockchain (hash).

Tous les votes / soutiens / propositions sont signés localement par le wallet (jamais la passerelle).

## 🗂️ Réorganisation des Crates
Nouveaux crates :
- `government-server/`
- `wallet-cli/` (ou `wallet-client/`)
  - `commands/`, `keystore.rs`, `attestation_flow.rs`, `signing.rs`, `config.rs`.

Refactors :
- `api-gateway/` : retirer validation documentaire directe → client HTTP vers government-server. Ajouter `attestations.rs`.
## 🗃️ Modèle de Données (API Gateway)
```sql
users(
  public_key BLOB UNIQUE,
  created_at TIMESTAMP,
  status TEXT,                 -- pending|validated|revoked
);

  raw_attestation JSON,
  issued_at TIMESTAMP,

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
| ISSUER_PUBKEY_EXPORT | blockchain-server | Chemin fichier où exporter la JWK publique (`GET /issuer/jwk`) |
| ALLOWED_ISSUERS_DIDS | blockchain-server | Liste CSV des DID émetteurs autorisés (ex: `did:key:...,did:key:...`) |
| MIGRATIONS_DIR | blockchain-server (tests) | Répertoire migrations SQL explicite pour les tests |
| BLOCKCHAIN_DATA_DIR | blockchain-server | Répertoire des données (contient `commitments.log`) |

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
# 🧾 État actuel (Identité & Pré‑ZKP livrés)

Cette section résume ce qui est déjà en place dans le workspace côté identité, engagements et outils de preuves (pré‑ZKP), afin d'aligner l'architecture avec l'état réel du code.

## Endpoints et flux livrés
- Issuer minimal intégré au nœud (`blockchain-server`):
  - `GET /issuer/jwk` exporte une JWK publique (kid déterministe), avec option d'export fichier si `ISSUER_PUBKEY_EXPORT` est défini.
  - `POST /issuer/credential` émet un VC (JSON-LD simplifié), signe avec Ed25519 et retourne le credential signé.
  - `POST /issuer/verify` vérifie un VC: reconstruction canonique, vérification signature, contrôle `verificationMethod`, allowlist `ALLOWED_ISSUERS_DIDS` et calcule le `commitment_hash` (SHA‑256 du JSON canonique sans `proof`).
- Pipeline d'engagement d'identité:
  - Lors de l'émission, insertion idempotente en base (hash d'engagement unique, statut `active`, dates d'émission/expiration, DIDs issuer/subject).
  - Au démarrage, le nœud hydrate en mémoire les engagements actifs depuis la DB et, si besoin, rétro-remplit `commitments.log`.
  - À la soumission/minage de transactions, `identity_ref` est validé (présence DB, statut `active`, non expiré, issuer autorisé) et les TX invalides sont filtrées avant minage.

## Fichier commitments.log et Merkle root/proofs
- Fichier append-only `commitments.log` (une ligne hex SHA‑256 par engagement), situé dans `${BLOCKCHAIN_DATA_DIR}/commitments.log` (par défaut `./data/commitments.log`).
- CLI `compute_root` (binaire du crate `blockchain-server`): calcule la racine de Merkle à partir du fichier.
- CLI `commitment_proof` (binaire):
  - `root` → imprime la racine hex;
  - `prove <index>` → génère une preuve JSON (siblings hex, `leaf_index`);
  - `verify <index> <proof.json>` → vérifie la preuve et affiche `valid=true|false`.
- Bibliothèque pré‑ZKP dans `common::identity::zkp_prelude`:
  - `IdentityAccumulator` (trait minimal), `MerkleAccumulator` (duplication du dernier nœud pour paires impaires), `MerkleProof`, `merkle_proof_for`, `verify_merkle_proof`.

## Outils complémentaires
- `issuer_key_tool` (binaire): génération/import/export de clé issuer Ed25519 avec `kid` déterministe et DID `did:key` dérivé; testé via import/round‑trip.
- `wallet-cli` (MVP): commandes VC (récupération, calcul de hash d’engagement, vérification facultative via `/issuer/verify`, publication côté nœud).

## Tests, benchs et garde‑fous
- Tests d’API issuer (`/issuer/verify`) positifs/négatifs, et E2E idempotence d’engagement côté nœud.
- Tests d’audit de confidentialité: pas de logs de données sensibles, pas de stockage de clé privée serveur.
- Intégration Merkle: parsing hex compatible `commitments.log`, génération et vérification de preuves contre la racine.
- Bench performance (Criterion) sur la vérification de VC (Partie 7.7).

## Documentation
- Diagramme identité et flux: `docs/identity_diagram.md`.
- Détails des preuves Merkle et de l’outil: `docs/merkle_proofs.md` (lié depuis le README).
- Aide au dépannage identité: `docs/troubleshooting_identity.md`.

### Essayer rapidement (preuves Merkle) 🧪
Préparez un fichier `commitments.log` actuel (le nœud le crée/alimente dans `./data/commitments.log`).

```powershell
# 1) Calculer la racine de Merkle du fichier courant
cargo run -p blockchain-server --bin compute_root -- ./data/commitments.log

# 2) Générer une preuve pour la feuille d'index 0 (adapter l'index selon votre fichier)
cargo run -p blockchain-server --bin commitment_proof -- ./data/commitments.log prove 0 > proof.json

# 3) Vérifier la preuve générée
cargo run -p blockchain-server --bin commitment_proof -- ./data/commitments.log verify 0 proof.json
```

Pour une description complète des formats, limitations (duplication du dernier nœud sur niveaux impairs) et API librairie, voir `docs/merkle_proofs.md`.

Voir aussi dans le README la section « Outils CLI (Merkle) » pour un aperçu des deux binaires: `compute_root` et `commitment_proof`.

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

---

## 🔎 Analyse de viabilité cryptographique, risques et méthodes proposées

Cette section regroupe une analyse de viabilité du sous-système cryptographique (attestations, commitments, ZKP) et propose plusieurs méthodes concrètes pour gérer les identités de façon décentralisée tout en préservant la vie privée.

### Synthèse exécutive — viabilité
- Le projet est viable et déjà bien avancé : gestion des VCs/DID, `commitments.log`, Merkle roots, et un prototype ZKP (Groth16 + Poseidon) sont présents.
- Points à surveiller avant un déploiement large : gestion du "trusted setup" (Groth16), synchronisation des fichiers Poseidon/VK entre wallet et nœud, performances de génération de preuve côté client, mécanismes de révocation.

### Points forts cryptographiques
- Signatures Ed25519 pour VC/attestations : robustes et standards.
- Canonisation JSON + SHA‑256 pour commitments : simple et reproductible.
- Utilisation de Poseidon pour circuits SNARK‑friendly.

### Risques et limites
1. Trusted setup (Groth16) — nécessité d'une cérémonie ou migration vers un schéma sans trusted setup.
2. Synchronisation des artefacts (Poseidon params, PK/VK) entre wallet et node — source fréquente d'échecs de vérification.
3. Temps/mémoire de génération de preuve sur clients mobiles (bench requis).
4. Revocation / suppression d'un engagement (append‑only `commitments.log` nécessite stratégie de révocation).
5. Point d'autorité sur l'émetteur (Government Server) — modèle hybride à clarifier.

### Recommandations techniques immédiates
- Automatiser la vérification d'intégrité des artefacts ZKP (hashes VK et Poseidon) et échouer si mismatch.
- Documenter et, si possible, organiser une cérémonie multi‑parties (Powers‑of‑Tau) pour le trusted setup ou évaluer PLONK/Halo2 pour réduire ce risque.
- Mesurer la performance de proof generation via `zkp_bench` sur plateformes représentatives (desktop / Android / iOS).
- Implémenter un mécanisme de révocation (revocation list, accumulator, ou marqueur d'invalidité en chaîne) et des tests E2E associés.

### Méthodes proposées pour la gestion décentralisée des identités (5 options)

Méthode A — Flow actuel (commitment + issuer atteste + Merkle root + ZKP membership)
- Contrat : VC signé → wallet calcule commitment → append au `commitments.log` → preuve ZK (membership + nullifier) pour actions.
- Avantages : déjà implémenté; bon compromis vie privée/praticabilité.
- Inconvénients : trusted setup (Groth16), synchronisation params, révocation complexe.

Méthode B — DID + Verifiable Credentials off‑chain + on‑chain minimal reference
- Contrat : VC W3C stocké off‑chain, on‑chain seulement le hash (commitment) et métadonnées minimales.
- Avantages : interopérabilité, faibles fuites d'information on‑chain.
- Inconvénients : revocation et vérification dépendent d'un registre/endpoint off‑chain.

Méthode C — Pairwise / per‑service pseudonymous DIDs + selective disclosure
- Contrat : Wallet dérive DID par service (HKDF/domaine), réduit linkability cross‑service; combine avec disclosure sélectif ou ZK.
- Avantages : limite traçage entre services.
- Inconvénients : gestion des seeds/backup, menace d'analyse de métadonnées.

Méthode D — Anonymous credentials (BBS+ / CL‑signatures) + revocation accumulator
- Contrat : Emission aveugle de crédentiels anonymes; wallet prouve possession/selective disclosure sans révéler identité.
- Avantages : forte confidentialité, pas de linkage entre présentations.
- Inconvénients : complexité d'implémentation, nécessité d'un système de révocation et de mises à jour d'accumulateur.

Méthode E — Décentraliser l'émission (multi‑issuer / threshold attestation)
- Contrat : attestations émises/validées par un quorum d'émetteurs (threshold signatures / multi‑sig) ; registre public des émetteurs.
- Avantages : pas de single point of failure, résilience politique et opérationnelle.
- Inconvénients : coordination entre autorités, UX plus lourde.

### Checklist opérationnelle (tests prioritaires)
1. Exécuter `zkp_bench` sur plateformes cibles et collecter latence/memoire.
2. Automatiser vérification de hash (Poseidon/VK) au boot wallet/server.
3. Planifier et documenter ceremony de setup ou évaluer alternative (PLONK/Halo2).
4. Conception et test d'un flux de révocation (E2E).
5. Rédiger model de menace et PIA (privacy impact assessment).

### Recommandations d'implémentation graduelle
- Si priorité = privacy forte : viser Méthode D (anonymous credentials) couplée à ZKP pour actions sensibles.
- Si priorité = pragmatique / low‑effort : conserver Méthode A et durcir ops (ceremony, sync automatisée, révocation simple).
- Si priorité = gouvernance et décentralisation : ajouter Méthode E (threshold issuers) et registre d'émetteurs sur chaîne.

### Prochaines actions que je peux implémenter / livrables
1. Ajouter un script PowerShell/CLI qui vérifie automatiquement que `poseidon_params.bin` et `vk‑*.bin` ont des hashes identiques entre wallet et server (PR automatique).
2. Lancer des benchs `zkp_bench` (je peux les exécuter ici si tu me fournis exemples de fichiers PK/VK/params ou m'autorises à générer des paramètres temporaires).
3. Rédiger un document comparatif Groth16 vs PLONK/Halo2 (trusted setup, proof size, temps proving/verif, maturité libs Rust).

---

## ✅ Résumé et décision attendue
Tu peux choisir la voie que tu souhaites prioriser :
- "Pragmatique" : stabiliser le flow actuel (A) + automations/ceremony.
- "Privacy‑max" : investir sur anonymous credentials (D) + revocation.
- "Décentralisation organisationnelle" : mettre en place multi‑issuer threshold (E).

Indique quelle option tu veux prioriser et je peux : (1) ouvrir une PR qui ajoute le script de vérification des paramètres, (2) lancer des benchs, ou (3) rédiger le plan de migration technique détaillé pour l'option choisie.

---

_Fin de l'ajout — analyse de viabilité, options et plan d'action intégrés._
