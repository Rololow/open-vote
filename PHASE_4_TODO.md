````markdown
# To-Do List - Phase 4 : Wallet CLI MVP & UX

**Objectif :** Compléter et durcir le `wallet-cli` pour supporter la production complète (gestion clés, VC, preuves ZKP, backup/restore, UX de base).

## Livrables principaux
- CLI stable (`generate-keypair`, `did-generate`, `vc-request`, `vc-commit`, `create-proposal`, `support`, `prove`)
- Keystore chiffré (Argon2id + AES-GCM)
- UX basique pour mobile / desktop (scripts/guide)
- Tests E2E : register → vc request → vc commit → anonymous action (si Phase 3 activée)

## Tâches proposées
- [x] 4.1 Finaliser `zkp_bench` intégration au `wallet-cli` pour benchs clients
- [x] 4.2 Implémenter keystore chiffré (`~/.e-gov-wallet/keys.enc`) + commandes backup/restore
- [x] 4.3 Sous-commandes VC : list, show, revoke-local
- [x] 4.4 UX: messages clairs en cas d'échec (params mismatch VK/Poseidon)
- [x] 4.5 Préparer mode headless pour CI (pas d'interaction passphrase)
- [x] 4.6 Tests unitaires & integration pour wallet flow

## ZKP: Intégration Poseidon production (Phase 4.7)

Objectif : Remplacer le gadget pédagogique Halo2 par un gadget Poseidon de production (Halo2-compatible) et fournir la génération/validation des paramètres Poseidon pour le champ Pasta (utilisé par Halo2).

Tâches détaillées :
- [x] 4.7.1 Choisir et ajouter une dépendance de gadget Poseidon compatible Halo2 (git pin pour cohérence avec `halo2_proofs`/`pasta_curves`).
- [x] 4.7.2 Implémenter l'adaptateur dans `wallet-cli/src/zkp_halo2.rs` : charger les paramètres Pasta, contraindre la permutation Poseidon multi-rounds en-circuit, exposer `membership_hash` et `nullifier_hash` comme inputs publics.
- [x] 4.7.3 Ajouter une commande CLI `ZkpGenPoseidonParams` (ou réutiliser la commande existante) pour générer et sauvegarder `poseidon_params.bin` (Pasta field) sous `<data_dir>/zkp/poseidon_params.bin`.
- [ ] 4.7.4 Tests :
	- [ ] Unittest natif vs gadget: comparer la sortie native Poseidon (crate) vs la sortie contrainte par le circuit.
	- [ ] MockProver test: prouver/verifier localement avec `zkp_halo2` feature.
	- [ ] E2E smoke: remplacer POC JSON par un proof envelope compatible et vérifier que le nœud accepte la preuve (mocked or local verifier).
- [ ] 4.7.5 Documentation: documenter la procédure de génération de paramètres et vérification des hachages de params (client/server) dans `docs/zkp_run.md` et `ARCHITECTURE_REDESIGN.md`.

Critères d'acceptation :
- Le gadget Poseidon compile et passe MockProver tests (`cargo test -p wallet-cli --features zkp_halo2`).
- La commande de génération de paramètres produit un `poseidon_params.bin` lisible par la CLI et le serveur, et leurs hachages peuvent être comparés.
- Les tests unitaires natif vs circuit passent.

## Critères de réussite
- CLI peut être utilisée en script CI pour un flow complet.
- Keystore chiffré avec test de round-trip backup/restore.

````