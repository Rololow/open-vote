# TODO Global — Refonte d'Architecture

Date: 2025-10-04

## Décision cruciale
- ZKP stack choisi : **Halo2** (préférence pour la récursion, aggregation et support Rust mature).

## Vue d'ensemble
Ce fichier centralise les tâches globales et pointe vers les TODOs détaillés par phase. Utiliser les fichiers de phase pour le découpage fin et les PRs correspondantes.

## Liens vers TODOs par phase
- Phase 1 — Décentralisation du point d'accès : `PHASE_1_TODO.md` (COMPLÉTÉE)
- Phase 2 — DID / VC : `PHASE_2_TODO.md`
- Phase 3 — ZKP (anonymous actions) : `PHASE_3_TODO.md` (prototype ok, adapter à Halo2)
- Phase 4 — Wallet CLI MVP & UX : `PHASE_4_TODO.md`
- Phase 5 — Transactions enrichies : `PHASE_5_TODO.md`
- Phase 6 — Durcissement & rotation clés : `PHASE_6_TODO.md`
- Phase 7 — Documentation finale & déploiement : `PHASE_7_TODO.md`

(ouvrir ces fichiers pour les tâches détaillées, checklists et critères d'acceptation)

## TODOs transverses (haut niveau)
1. Automatiser vérification des artefacts ZKP (Poseidon params & VK hashes) — script + CI check.
2. Benchmarks comparatifs prover time / memory sur Halo2 (utiliser `zkp_bench` comme base de comparaison).
3. Mettre en place procédure SRS/params (documentation, vérification, signature des artefacts).
4. Implémenter révocation (design revocation list vs accumulator) et tests E2E.
5. CI: jobs pour `identity` feature on/off, tests ZKP, et collecte d'artefacts (`errors.log`, vk hash).
6. Monitoring & alerting : mismatch params, verification fails, high prover latency.

## Priorités (court terme)
- P0 : Script de vérification des params VK/Poseidon + ajouter check dans CI (automatiser avant chaque run E2E).
- P1 : Adapter l'implémentation ZKP actuelle vers Halo2 (proof of concept minimal).
- P2 : Bench Halo2 sur circuit Poseidon/merkle (desktop et mobile si possible).
- P3 : Finaliser Wallet CLI (Phase 4) pour l'intégration utilisateur et tests E2E.

## Règles d'usage
- Chaque ticket/PR doit référencer le fichier de phase correspondant.
- Mettre à jour ce `todo global.md` quand une phase passe `COMPLÉTÉE`.
- Pour toute modification des artefacts ZKP (vk/pk/params) : publier le hash et noter la version (`vk_version`) dans le changelog.

## Contacts / Mainteneurs
- Architecture & ZKP : équipe core (voir `ARCHITECTURE_REDESIGN.md` pour responsables suggérés)
- Wallet & UX : équipe client
- Opérateurs / CI : équipe devops

---

Si tu veux, j'ajoute maintenant :
- le script PowerShell de vérification des artefacts (PR), ou
- un bench Halo2 minimal dans `wallet-cli` (POC) pour mesurer les temps de proving.

Choisis l'action que tu veux prioriser et je l'implémente.