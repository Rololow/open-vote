# Phase 4 — Compte rendu

Objectif de la phase
- Stabiliser et enrichir le wallet-cli pour un usage en production: gestion de clés sécurisée (keystore chiffré), UX de base, commandes VC, préparation ZKP.

Livrables réalisés
- Keystore chiffré (nouveau)
  - Fichier: ~/.e-gov-wallet/keys.enc (chemin par défaut; surchargable via --store)
  - Chiffrement: AES-256-GCM (nonce aléatoire 96-bit)
  - Dérivation de clé: Argon2id (m=19456, t=2, p=1, version 0x13), salt aléatoire
  - Format: header MAGIC (EGOVKS\x01) + longueur + JSON enveloppe (kdf params, nonce, ciphertext). Le plaintext JSON contient les entrées (id, public_key_hex, private_key_hex)
  - API CLI (nouvelles sous-commandes):
    - keystore-init: initialise un keystore vide chiffré
    - keystore-list: liste les clés (id, clef publique)
    - keystore-import: importe une clé privée (hex 32 octets) + clé publique, avec id optionnel
    - keystore-export: exporte une entrée (utile pour migration/backup contrôlé)
    - keystore-remove: supprime une clé par id
    - keystore-backup: copie du keystore chiffré vers un fichier
    - keystore-restore: restaure un keystore à partir d’un backup

- Commandes VC (UX et gestion locale)
  - vc-list: inventorie les VC stockés localement avec métadonnées (issuer, subject, issuance/expiration)
  - vc-revoke-local: révocation locale (suppression du fichier) par --file ou --hash

- Améliorations UX ZKP (Groth16)
  - Messages explicites si la génération de preuve échoue (rappels pour générer/synchroniser poseidon_params.bin et emplacement attendu)
  - La CLI journalise le hash des paramètres Poseidon afin de comparer facilement avec le nœud (synchronisation)

Changements de code principaux
- wallet-cli/src/keystore.rs (nouveau): implémentation du keystore chiffré (Argon2id + AES-GCM), format, et API
- wallet-cli/src/main.rs: intégration des sous-commandes keystore; ajout des sous-commandes vc-list et vc-revoke-local; amélioration des messages ZKP en cas d’erreur
- wallet-cli/src/identity.rs: nouvelles fonctions vc_list et vc_revoke_local
- wallet-cli/Cargo.toml: ajout des dépendances argon2, aes-gcm, zeroize, base64

Sécurité
- Aucune clé en clair stockée hors du keystore; les exports sont possibles uniquement à la demande via keystore-export
- Paramètres KDF raisonnables pour desktop; ajustables à l’avenir si besoin (mode mobile/CI)
- zeroize utilisé pour nettoyer des buffers sensibles sur chemins critiques

Utilisation — Exemples
- Initialiser un keystore:
  - wallet-cli keystore-init --passphrase "votre_passphrase"
- Importer une clé existante:
  - wallet-cli keystore-import --passphrase "votre_passphrase" --private-key-hex 001122.. --public-key-hex <pub_hex>
- Lister / exporter / supprimer:
  - wallet-cli keystore-list --passphrase "votre_passphrase"
  - wallet-cli keystore-export --passphrase "votre_passphrase" --id <id>
  - wallet-cli keystore-remove --passphrase "votre_passphrase" --id <id>
- Backup / restore:
  - wallet-cli keystore-backup --out ./backup.keys.enc
  - wallet-cli keystore-restore --backup ./backup.keys.enc
- VC gestion locale:
  - wallet-cli vc-list --dir ~/.e-gov-wallet/credentials
  - wallet-cli vc-revoke-local --hash <commitment_hash>
- ZKP (si feature activée):
  - wallet-cli zkp-gen-poseidon-params --data-dir ./wallet-cli
  - wallet-cli zkp-prove --data-dir ./data --vk-version 1 --root-hex <hex> --scope <scope> --nullifier-hex <hex> --leaf-hex <hex> --secret-hex <hex> --sib <hex> ... --directions 0101... --out zkp_proof.json

État par rapport aux critères de réussite (Phase 4)
- Keystore chiffré: implémenté, avec backup/restore et cycle import/export/list/remove
- CLI utilisable en CI: base en place; prochaine étape recommandée d’injection passphrase via variable d’environnement et mode non interactif across commands

Dettes/Prochaines étapes
- Intégrer le keystore dans les flux de signature existants
  - Ajouter --key-id sur create-proposal, anonymous-*, vc-* pour charger la clé depuis le keystore (keystore::load_private_key)
  - Option passphrase par variable d’environnement (WALLET_PASSPHRASE) quand --passphrase n’est pas fourni (mode headless/CI)
- Tests
  - Unitaires: round-trip init → import → list → export → remove → backup → restore, cases d’erreur (passphrase incorrecte, fichier tronqué, id dupliqué)
  - Intégration: vc-request → vc-commit → vc-list → vc-revoke-local, ZKP positif/négatif (params mismatch)
- Documentation
  - Etendre README/PHASE_4_TODO avec la marche à suivre keystore et bonnes pratiques (gestion passphrase, sauvegarde)
- ZKP/Halo2 (Phase 4.7)
  - Sélection du gadget Poseidon compatible halo2_proofs/pasta_curves (version pin)
  - Implémentation gadget et tests MockProver + natif vs circuit
  - Génération/synchronisation des paramètres Poseidon pour Pasta et doc d’alignement client/serveur

Résumé
- Le keystore chiffré est en place avec un ensemble complet de commandes.
- Le wallet-cli a gagné en UX (VC list/revoke, messages ZKP) et se rapproche d’un usage CI/production.
- Les prochaines itérations visent l’intégration du keystore dans les commandes de signature, l’industrialisation headless/CI, et la poursuite de l’effort ZKP (Halo2 Poseidon).

---

Intégration keystore dans les flux de signature (nouveau)

Fonctionnement
- Un helper a été ajouté pour résoudre la clé privée depuis le keystore (si --key-id est fourni) ou depuis un fichier (fallback), afin d’éviter d’exposer des clés en clair en production.
- Les commandes supportent désormais: --key-id, --passphrase, --store.
- Mode CI/headless: si --passphrase est omis, la passphrase est lue depuis la variable d’environnement WALLET_PASSPHRASE.

Commandes mises à jour
- Créer une proposition (signature via keystore):
  - CLI: wallet-cli create-proposal --title "..." --description "..." --key-id <id> --passphrase "..." --node-url http://localhost:3000
  - CI: set WALLET_PASSPHRASE=... puis wallet-cli create-proposal --title ... --description ... --key-id <id>
- Dériver un DID depuis le keystore:
  - wallet-cli did-generate --key-id <id> --passphrase "..."

Paramètres de compatibilité
- Si --key-id n’est pas fourni, le comportement existant avec --key-file est conservé.
- WALLET_PASSPHRASE n’est lue que si --passphrase est absent, pour éviter les surprises.

Exemples de scénarios CI
- Windows PowerShell:
  - $env:WALLET_PASSPHRASE = "S3cret!"
  - cargo run -p wallet-cli -- create-proposal --title "Test" --description "CI" --key-id mykey --node-url http://127.0.0.1:3000
- POSIX:
  - export WALLET_PASSPHRASE='S3cret!'
  - cargo run -p wallet-cli -- create-proposal --title "Test" --description "CI" --key-id mykey --node-url http://127.0.0.1:3000

Tests recommandés
- Unitaires/intégration keystore:
  - init → import → create-proposal (--key-id) → succès
  - did-generate via --key-id (avec et sans --passphrase, avec WALLET_PASSPHRASE)
  - Négatifs: --key-id sans passphrase et sans WALLET_PASSPHRASE (doit échouer clairement), id introuvable, keystore corrompu
- Intégration complète (smoke):
  - wallet-cli keystore-init/import → did-generate --key-id → vc-request → vc-commit → create-proposal --key-id

Bonnes pratiques
- Ne pas commiter de passphrase dans les scripts; utiliser des variables d’environnement injectées par le runner CI.
- Préférer --key-id plutôt que --key-file en production.
- Sauvegarder régulièrement le keystore chiffré (keystore-backup) et tester la restauration (keystore-restore).

Nouvelle sous-commande — keystore-generate-keypair
- Objectif: générer une paire Ed25519 et l’importer directement dans le keystore (aucun fichier clé en clair sur disque)
- Usage:
  - wallet-cli keystore-generate-keypair --passphrase "votre_pass"
  - wallet-cli keystore-generate-keypair --passphrase "votre_pass" --id "prod-key-01" --store ~/.e-gov-wallet/keys.enc
- Sortie: affiche id, clé publique et did:key dérivé. Utiliser ensuite --key-id <id> pour signer.

Script CI headless (Windows PowerShell)
- Fichier: scripts/ci_wallet_headless.ps1
- Paramètres:
  - -NodeUrl: URL du nœud (défaut http://127.0.0.1:3000)
  - -IssuerEndpoint: URL issuer (optionnel) pour vc-request/commit
  - -Store: chemin du keystore (défaut ~/.e-gov-wallet/keys.enc)
  - -Passphrase: passphrase du keystore (obligatoire)
  - -KeyId: id de clé (optionnel; si absent, le script génère et importe une clé)
  - -CredDir: répertoire des VC (défaut ~/.e-gov-wallet/credentials)
- Comportement:
  - Initialise le keystore (idempotent)
  - Génère + importe une clé si non fournie
  - Dérive le did:key depuis --key-id
  - Optionnel: vc-request + vc-commit
  - Si le nœud est joignable: envoie une proposition signée via --key-id
- Exemples:
  - pwsh -NoProfile -File scripts/ci_wallet_headless.ps1 -Passphrase "S3cret!" -NodeUrl http://127.0.0.1:3000
  - pwsh -NoProfile -File scripts/ci_wallet_headless.ps1 -Passphrase "S3cret!" -IssuerEndpoint http://127.0.0.1:8095 -NodeUrl http://127.0.0.1:8095
