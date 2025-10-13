````markdown
# To-Do List - Phase 4 : Wallet CLI MVP & UX

**Objectif :** Compléter et durcir le `wallet-cli` pour supporter la production complète (gestion clés, VC, preuves ZKP, backup/restore, UX de base).

## Livrables principaux
- CLI stable (`generate-keypair`, `did-generate`, `vc-request`, `vc-commit`, `create-proposal`, `support`, `prove`)
- Keystore chiffré (Argon2id + AES-GCM)
- UX basique pour mobile / desktop (scripts/guide)
- Tests E2E : register → vc request → vc commit → anonymous action (si Phase 3 activée)

## Tâches proposées
- [ ] 4.1 Finaliser `zkp_bench` intégration au `wallet-cli` pour benchs clients
- [ ] 4.2 Implémenter keystore chiffré (`~/.e-gov-wallet/keys.enc`) + commandes backup/restore
- [ ] 4.3 Sous-commandes VC : list, show, revoke-local
- [ ] 4.4 UX: messages clairs en cas d'échec (params mismatch VK/Poseidon)
- [ ] 4.5 Préparer mode headless pour CI (pas d'interaction passphrase)
- [ ] 4.6 Tests unitaires & integration pour wallet flow

## Critères de réussite
- CLI peut être utilisée en script CI pour un flow complet.
- Keystore chiffré avec test de round-trip backup/restore.

````