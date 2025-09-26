# To-Do List - Phase 2 : Intégration des Standards d'Identité (DID / VC)

**Objectif Stratégique :** Remplacer le système d'attestation propriétaire par des **Identifiants Décentralisés (DID)** et des **Crédentiels Vérifiables (Verifiable Credentials - VC)** afin de préparer l'anonymisation (Phase 3) et d'éliminer le point de défaillance unique.

**Statut :** 🚧 En cours – Issuer opérationnel (signature réelle, did:key). Commitment hash + stockage idempotent en DB avec migrations appliquées (Partie 3 terminée). Endpoint `/issuer/verify` implémenté et clé issuer persistée sur disque. Côté nœud: validation `identity_ref` en place à la soumission (mempool), au minage (pré-filtrage) et lors de la réception de blocs, plus endpoint public `GET /identity/commitments/:hash`. Sous-commandes wallet (did/vc) en place et endpoint explicite `POST /api/identity/commit` implémenté. Tests d’intégration négatifs ajoutés (signature invalide, digest mismatch, VC expiré). E2E étendu: soumission d’une transaction avec `identity_ref` et minage vérifié. 

**Sortie Attendue de la Phase 2 :**
- Un émetteur de crédentiels ("issuer") opérationnel (service ou module) capable d'émettre un VC signé Ed25519 (format JSON-LD ou JWT).
- Le wallet peut : (1) demander un VC, (2) le stocker localement, (3) le présenter ou en dériver un hash d'engagement.
- La blockchain accepte des transactions qui référencent un VC (directement ou via hash) et vérifie la signature + l'émetteur.
- Schéma de migration DB minimal pour stocker un hash / engagement d'identité (pas le VC brut si évitable).
- Tests d'émission / vérification / rejet (émetteur inconnu, signature invalide, VC expiré).

---
## 📊 Résumé des Axes de Travail
| Axe | Description | Livrable | Priorité |
|-----|-------------|----------|----------|
| A | Choix & intégration bibliothèque DID/VC | Dépendance `ssi` / alternative validée | Haute |
| B | Émetteur de VC (issuer) | Service ou module `issuer` + clé persistée | Haute |
| C | Modélisation & stockage | Table / colonne pour engagements identité | Haute |
| D | Mise à jour wallet | Sous-commandes VC (fetch, list, show) | Haute |
| E | Validation côté nœud | Vérification VC & origine émetteur | Haute |
| F | Transactions enrichies | Champ optionnel `identity_ref` / `identity_commitment` | Moyenne |
| G | Tests & conformité | Tests unit / intégration / négatifs | Haute |
| H | Sécurité & rotation clés | Procédure génération / rotation | Moyenne |
| I | Préparation Phase 3 | Hook pour future ZKP (commitment set) | Moyenne |

---
## 🧱 Partie 1 : Choix & Intégration Bibliothèque DID/VC
Objectif : Introduire une couche d'abstraction minimale pour ne pas lier fortement le code à une lib spécifique.

 - [x] 1.1 Étudier `ssi` (analyse initiale + intégration feature-gated)
 - [x] 1.2 Vérifier poids dépendances & compatibilité MSRV / build Windows (compilation réussie Windows, benchs OK)
 - [x] 1.3 Créer module `common/src/identity/` :
   - [x] `mod.rs` (re-exports)
   - [x] `did.rs` (types internes minimalistes – `Did`)
   - [x] `vc.rs` (wrapper `CitizenCredentialWrapper`, parse + extraction issuer/subject + stub verify)
 - [x] 1.4 Ajouter feature flag `identity` (build sélectif)
 - [~] 1.5 Définir format interne neutre (struct Rust) + conversions vers VC JSON-LD / JWT
   - État: wrapper interne & DID ok, conversion explicite vers JSON-LD/JWT non encore implémentée (reportée Partie 2 après signature réelle)
- [x] 1.6 Choisir stratégie canonisation JSON (impl tri manuel provisoire)
- [x] 1.7 Implémenter `canonical_json_str(raw: &str)`
- [x] 1.8 Implémenter `compute_credential_hash(...)` + helpers
- [x] 1.9 Test stabilité (réordonnancement) — effectué (`hash_stable_reordered`)
- [x] 1.10 Doc `docs/identity_hash.md` créée
- [x] 1.11 Stub `verify_signature` enrichi d'un warning (`did-resolution-disabled (stub)`)
  - [x] 1.12 Bench (criterion) canonisation + hash (objectif < 1ms) — baseline: canonical ~7.28–7.69 µs; hash ~7.42–7.62 µs (Windows bench profile, sample VC, 100 samples)
  - [x] 1.13 Test golden hash sur VC exemple (`golden_hash_example_vc`) — garantit non-régression canonisation

**Critères Acceptation :** `cargo check --no-default-features` passe & build complet avec feature `identity` passe.

---
## 🏛️ Partie 2 : Service Émetteur (Issuer)
Deux options :
1. Nouveau crate `issuer-server/` (plus propre, extensible multi-émetteurs)  
2. Module activable dans `blockchain-server` (plus rapide).  

=> **Choix proposé (rapide Phase 2)** : Module `issuer` dans `blockchain-server` (implémenté – structure simplifiée).

- [x] 2.1 Créer module `issuer` (clé + construction VC) — fichier unique `issuer/mod.rs` (routes séparées dans `api/issuer_api.rs`)
- [x] 2.2 Génération clé Ed25519 persistée si absente (`ISSUER_KEY_PATH`) — reload encore stub (TODO restauration réelle)
- [x] 2.3 Endpoint `POST /issuer/credential` — conversion effectuée (payload `{ subject_did }` + validation minimale)
- [x] 2.4 Claims minimales VC (types, subject DID placeholder, attributs basiques) implémentées
- [x] 2.5 Validité configurable (`ISSUER_VC_VALIDITY_DAYS`) appliquée
- [x] 2.6 Endpoint JWK public: `GET /issuer/jwk` (export public key stub base64url)
 - [x] 2.7 Logging issuance (commitment hash + digest + signature prefix, pas de données sensibles)

**Critères Acceptation :** Requête wallet → VC signé → vérifiable via module common.

---
## 💾 Partie 3 : Modélisation & Base de Données
Objectif : Stocker seulement un engagement (hash) ou un identifiant de VC, pas le VC complet (privacy by design).

- [x] 3.1 Ajouter table (ou étendre existante) `identity_commitments` :
```
identity_commitments(
  id INTEGER PK,
  public_key TEXT,             -- clé publique du wallet (legacy compat)
  did TEXT,                    -- DID complet (did:key:...)
  commitment_hash TEXT UNIQUE, -- hash du VC ou d'un engagement dérivé
  issuer_did TEXT,
  issued_at TIMESTAMP,
  expires_at TIMESTAMP,
  status TEXT DEFAULT 'active' -- active|revoked|expired
);
```
- [x] 3.2 Script migration (SQL) + version bump + chargeur de migrations (dossiers migrations/*.sql)
- [x] 3.3 Générer `commitment_hash = sha256(canonical(vc_without_signature))` (intégré via réutilisation `artifacts.canonical_json` dans endpoint issuance)
- [x] 3.4 Index sur `(did)` + `(issuer_did)`
- [x] 3.5 Ajout vérification unicité (constraint UNIQUE sur `commitment_hash`)

**Critères Acceptation :** Rejeu même VC → réponse idempotente (retour en 200 + identique) / hash différent → nouvelle ligne.

---
## 🔐 Partie 4 : Mise à Jour Wallet CLI
- [x] 4.1 Nouveau sous-module `wallet-cli/src/identity.rs`
- [ ] 4.2 Sous-commandes :
  - [x] `wallet vc request --endpoint <url>` → récupère & sauvegarde `citizen_credential.json`
  - [x] `wallet vc show` → affiche meta (issuer, exp, subject DID)
  - [x] `wallet vc commit --node <url>` → calcule hash & POST `/identity/commit`
  - [x] `wallet did generate` (si pas déjà fait : derive did:key de la clé existante)
- [x] 4.3 Stockage local : `~/.e-gov-wallet/credentials/<commitment_hash>.json`
- [x] 4.4 Normalisation canonical JSON (tri clés) avant hash
- [x] 4.5 Vérification expiration locale avant commit

**Critères Acceptation :** Flow complet scriptable sans interaction manuelle (hors passphrase futur).

---
## ✅ Partie 5 : Validation Côté Nœud Blockchain
- [x] 5.1 Ajouter liste des émetteurs autorisés (pour l'instant un seul) : fichier `issuers.json` ou variable `ALLOWED_ISSUERS_DIDS`
- [x] 5.2 Vérification des transactions contenant `identity_ref` :
  1. Lookup `commitment_hash` en DB
  2. Vérifier non expiré & status=active
- [x] 5.3 Rejeter si émetteur inconnu
- [x] 5.4 Préparer hook pour Phase 3 : ajouter dans un set mémoire `ACTIVE_COMMITMENTS` (source future Merkle root)
  - Hydratation au démarrage depuis la DB (status=active) avec filtrage des expirations, et backfill optionnel de `commitments.log` si absent
- [x] 5.5 Exposer endpoint `GET /identity/commitments/:hash` (lecture publique minimaliste)

**Critères Acceptation :** Transaction avec commitment inconnu → rejet explicite.

---
## 📦 Partie 6 : Extension des Transactions
- [x] 6.1 Ajouter champ optionnel `identity_ref` ou `identity_commitment: Option<[u8;32]>` dans types `Transaction` (ou nouvelle struct `SignedIdentityRef` si isolation nécessaire)
- [ ] 6.2 Migration sérialisation (versioning des transactions si nécessaire) — non requis à ce stade (champ optionnel conservant la compatibilité)
- [x] 6.3 Backward compat : ancienne transaction sans champ → toujours acceptée pour tests (flag de config pour exiger identity)
- [x] 6.4 Ajouter test sérialisation (hex golden file)

**Critères Acceptation :** Ancien wallet (Phase 1) peut encore soumettre une transaction simple si `ALLOW_LEGACY_TX=1`.

---
## 🧪 Partie 7 : Tests Unitaires & Intégration
- [x] 7.1 Unit/Int: parsing & vérification VC (signature alt → échec)
- [x] 7.2 Unit: hash canonical stable (réordonner champs → même hash) (déjà couvert par tests Partie 1 + golden hash)
- [x] 7.3 Int: flow complet (generate key → request vc → commit → submit tx avec identity_ref)
- [x] 7.4 Int: VC expiré → rejet commit
 - [x] 7.5 Int: issuer inconnu → rejet
  - [x] 7.6 Int: double commit même VC → idempotent (test d'idempotence existant)
- [ ] 7.7 Bench (optionnel) : coût vérif VC (< X ms cible)
  - [x] (Nouveau) 7.8 Int: endpoint /issuer/verify une fois implémenté

Notes tests supplémentaires réalisés:
- Unités côté nœud: rejet `identity_ref` inconnu/expired/disallowed; filtrage au minage (transactions révoquées supprimées), mix valide+invalide miné correctement. Tous les tests passent (voir `blockchain-server/src/node.rs` tests).

---
## 🔄 Partie 8 : Sécurité & Gestion Clés Issuer
- [ ] 8.1 Génération clé Ed25519 offline (script) + import
- [ ] 8.2 Rotation clé (préparer champ `kid` dans JWK)
- [ ] 8.3 Empêcher log accidentel du VC complet (audit logging => hash seulement)
- [ ] 8.4 Vérifier absence de stockage clés privées côté DB (scan code)
- [ ] 8.5 Variable d'env : `ISSUER_KEY_PATH`, `ISSUER_PUBKEY_EXPORT`

**Critères Acceptation :** Redémarrage service n'invalide pas VC précédents.

---
## 🧭 Partie 9 : Préparation Phase 3 (ZKP)
- [ ] 9.1 Ajouter module `zkp_prelude.rs` (placeholder) avec trait `IdentityAccumulator`
- [ ] 9.2 Enregistrer chaque `commitment_hash` dans un fichier append-only (`commitments.log`)
- [ ] 9.3 Script utilitaire génère Merkle root depuis log (rust binaire `tools/compute_root.rs`)
- [ ] 9.4 Documenter format log (une ligne = hex(hash))

**Critères Acceptation :** Génération Merkle root reproductible sur deux environnements.

---
## ⚙️ Partie 11 : Scripts & Automatisation
- [x] 11.1 Script PowerShell `scripts/identity_flow.ps1` exécutant flow VC (did gen → vc request → commit → tx)
- [ ] 11.2 Script bash équivalent (pour CI Linux) `scripts/identity_flow.sh`
- [ ] 11.3 Job CI (GitHub Actions futur) : matrice `{ identity=on, identity=off }` sur `cargo check` + tests
- [ ] 11.4 Ajout d'un binaire utilitaire interne `tools/canonical_check.rs` (valide hash d'un VC passé en argument)
- [ ] 11.5 Tâche cargo alias: `[alias] vc-hash = "run -p common --features identity --example hash_vc"`
- [ ] 11.6 Vérification automatique absence de fuite (grep sensible) dans CI (`VC` / `credentialSubject` non loggé)

**Critères Acceptation :** Un contributeur peut lancer `./scripts/identity_flow.*` et obtenir un hash d'engagement + transaction acceptée (stub pour l'instant).

---
## 🗂️ Partie 10 : Documentation & DX
- [ ] 10.1 Mise à jour `ARCHITECTURE_REDESIGN.md` section identité (remplacer attestation propriétaire)
- [x] 10.2 Nouveau README segment "Cycle de Vie d'un VC"
- [ ] 10.3 Ajouter schéma PlantUML / Mermaid (issuer ↔ wallet ↔ blockchain)
- [ ] 10.4 Guide troubleshooting (erreurs fréquentes : signature invalide, issuer inconnu, hash mismatch)
- [ ] 10.5 Exemple JSON VC minimal dans `docs/examples/vc_citizen.json`

**Critères Acceptation :** Nouveau contributeur peut exécuter le flow en < 10 min via doc.

---
## 🧨 Risques & Mitigations
| Risque | Impact | Mitigation |
|--------|--------|-----------|
| Bibliothèque VC lourde / lente | Build & perf | Isoler derrière feature flag + bench |
| Invalidation future format ZKP | Refactor coûteux | Abstraction commitment stable dès maintenant |
| Fuite VC dans logs | Vie privée | Audit logging + tests recherche motifs |
| Clé issuer compromise | Confiance | Prévoir rotation + révocation (liste VC invalidés) |
| Hash canonical instable | Collisions logiques | Définir procédure canonisation stricte + test golden |

---
## 📌 Critères de Sortie (Definition of Done Phase 2)
- Tous les items Axe A→G marqués complétés.
- Script e2e (Make/PowerShell) exécute le flow complet sans intervention.
- Couverture tests identity ≥ 80% lignes des nouveaux modules.
- Documentation mise à jour & relue.
- Préparation Phase 3 (commitments log + outil root) disponible.

---
## 🧪 Commande E2E (But Cible Futur – Indicatif)
```
wallet did generate \
  && wallet vc request --endpoint http://localhost:8080/issuer/credential \
  && wallet vc commit --node http://localhost:8080 \
  && wallet create-proposal --title "Réforme X" --body "Texte..." --node http://localhost:8080
```

---
## 🔄 Plan d'Intégration Progressive
1. (Parties 1 & 2) => Base VC & issuer
2. (Parties 3 & 4) => Commitments & wallet flow
3. (Parties 5 & 6) => Validation nœud + extension transactions
4. (Partie 7) => Tests & robustesse
5. (Parties 8 & 9) => Sécurité + préparation ZKP
6. (Partie 10) => Documentation finalisation

---
## ✅ Prochaines Actions Immédiates (Sprint 2 Phase 2 - mise à jour)
### Complété ce sprint précédent
- Implémentation dérivation réelle `did:key` (multicodec 0xED01 + base58btc)
- Signature Ed25519 réelle + preuve prototypée
- Calcul & insertion `commitment_hash` (idempotence garantie via UNIQUE)
- Logging issuance (hash + digest + prefix signature)
- Test intégration idempotence

### À faire maintenant
- [x] Script PowerShell `identity_flow.ps1` (request VC → hash local → submit → mine) (11.1)
- [x] Endpoint `/issuer/verify` (validation signature + dates + hash recompute)
- [x] Persistance durable clé issuer (ne plus régénérer à chaque démarrage) + test redémarrage
- [x] Migration SQL dédiée (3.2) pour `identity_commitments` (fichier versionné)
- [x] Liste émetteurs autorisés (5.1) + validation côté transactions (soumission/minage/réception)
- [x] Sous-commandes wallet CLI (4.x): `did generate`, `vc request`, `vc show`, `vc commit` (persistance locale)
- [x] Endpoint lecture `GET /identity/commitments/:hash` (5.5)
- [x] Tests d'intégration négatifs `/api/identity/commit`: signature invalide, digest mismatch, VC expiré (7.4/7.5)
- [x] E2E intégration: submit tx avec `identity_ref` et minage (7.3)
- [x] `wallet-cli create-proposal --identity-hash` pour attacher `identity_ref` via /rpc
- [x] Tests unitaires nœud: rejet unknown/expired/disallowed + filtrage mempool/minage

### Étape suivante proposée
- [x] 4.5 Vérification expiration locale avant `vc commit` (wallet) — fail-fast
- [x] 11.2 Script bash équivalent `scripts/identity_flow.sh` (CI Linux)
- [x] 5.4 Préparer hook Phase 3: set mémoire `ACTIVE_COMMITMENTS` (source future Merkle)
- [ ] 9.2/9.3 Journal append-only `commitments.log` + outil Merkle root (`tools/compute_root.rs`)
- [ ] 10.3 Diagramme (issuer ↔ wallet ↔ blockchain) + 10.4 Guide troubleshooting
- [ ] 7.1 Unit tests VC parsing/verify (niveau lib) pour compléter la couverture

**En cours / à enchainer immédiatement.**
