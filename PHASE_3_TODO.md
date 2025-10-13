# Phase 3 — TODO (Anonymous Actions with ZKP)

Date: 2025-09-29

## Status update (live)

 - ✅ **E2E automation success (2025-10-01)**: I ran the end-to-end automation script that builds the wallet and server, generates a proof, POSTs it to the node and verifies server-side. The flow completed successfully and temporary artifacts were produced during the run:
   - Proof envelope: C:\Users\waric\AppData\Local\Temp\zkp-flow-*/zkp_proof.json
   - Proof binary: C:\Users\waric\AppData\Local\Temp\zkp-flow-*/zkp_proof.bin
   - Data directory used by the run: C:\Users\waric\AppData\Local\Temp\zkp-flow-*
   (Files and paths are environment-specific; the run logs were written to `errors.log` during verification.)
 
## Statut finalisé - Phase 3 (synthèse)

- ✅ Prototype fonctionnel : circuit Poseidon-based R1CS et preuve Groth16 (prototype) intégrés.
- ✅ Wallet CLI : génération de preuve (prove_membership_nullifier) testée via `zkp_bench`/binaire de bench.
- ✅ Endpoint serveur : `POST /api/identity/verify_zkp` disponible et vérifie les preuves avec VK versioning.
- ✅ E2E automatisé : script `scripts/zkp_flow_with_clean_log.ps1` exécute build → proof → POST → verify et écrit diagnostics dans `errors.log`.
- ✅ Merkle roots & commitments : `commitments.log`, calcul de root et fourniture de proofs via API fonctionnels.
- ✅ Transactions : nouveaux types `AnonymousVote` / `AnonymousSupport` (feature-gated) et pipeline de vérification mempool → minage intégrés.

## Points restants / priorités (court terme)

1. Migrer le prototype Groth16 vers **Halo2** (POC) — adapter circuits et harness de proving; garder Groth16 comme fallback si nécessaire.
2. Benchmarks comparatifs (Prover time, memory) sur Halo2 vs Groth16 sur hardware représentatif (desktop + mobile).
3. Robustesse : implémenter les contrôles négatifs côté serveur (params mismatch, root obsolète, nullifier duplicate) et ajouter métriques/tracing sur la vérification.
4. Revocation / root anchoring policy : définir fenêtre d'acceptation des roots et procédure de révocation d'engagements.
5. CI : ajouter checks pour l'intégrité des artefacts ZKP (VK/Poseidon hash) et tests négatifs automatisés.

## TODOs (révisés)
- [x] Vérifier la synchronisation des paramètres Poseidon et VK entre client et serveur.
- [x] Exposer latest root et historique, ancrer root dans les blocks.
- [x] Ajouter tx types anonymous (feature-gated) et pipeline de vérification.
- [x] E2E automation script et tests d'intégration (passés).
- [ ] Migrer POC → Halo2 et valider performance / compatibilité.
- [ ] Ajouter negative tests & CI checks (parameter integrity, invalid proofs).
- [ ] Bench et collecte métriques (verify/prove timings) pour décision finale.

This document outlines the goals, design decisions, tasks, milestones, and risks for implementing anonymous-but-verifiable actions using zero-knowledge proofs (ZKP), building on the identity commitments and Merkle groundwork from Phase 2.

## Goals and success criteria
- Anonymous participation: citizens can act (vote/support/propose) without revealing identity, while enforcing one-person-one-action per scope.
- Deterministic, auditable node-side verification, with bounded CPU/memory cost.
- Smooth wallet UX: generate proofs against latest commitment root, handle root drift gracefully, and avoid double-spend via nullifiers.

Success is when:
- Wallet can produce a membership proof + scoped nullifier for at least one action (vote or support).
- Node verifies the proof and rejects duplicates via nullifier checks.
- End-to-end test passes: wallet → proof → node → block inclusion.

## Early design decisions to finalize
- ZKP stack choice: recommend Halo2 (Plonkish, KZG) or Arkworks + Groth16. Preference: Halo2 for a modern proving system; note fallback to Groth16 for speed.
- Accumulator strategy: keep Merkle accumulator v1 (duplicate-last rule) for continuity; ensure root anchoring policy (per-block or periodic) and storage of historical roots.
- Nullifier scheme: deterministic nullifier per (action_scope, subject secret); collision-resistant; include chain/domain separation. Define replay window and invalidation rules.
- Circuit boundaries: at minimum prove (a) membership in current/known root, (b) correct nullifier derivation for the scope, (c) optional freshness/expiry checks.

## Work items
### Synchronisation & Robustesse Client/Serveur ZKP
- [x] Vérifier la synchronisation des paramètres Poseidon et VK entre client et serveur (même source, version, hash).  
  Status: verified during E2E automated run — poseidon params and VK file hashes matched between wallet and server in the successful flow.
- [ ] Implémenter des contrôles stricts côté client sur la génération du secret et du leaf (liés à l'identité, non réutilisables).
- [ ] Côté serveur, ajouter une vérification explicite que le leaf correspond à une identité valide (ex : hash de la carte).
- [x] Ajouter des tests croisés : preuve générée sur le client doit être systématiquement vérifiable sur le serveur, et inversement.
  Status: cross-test proved in the integration flow and unit tests; keep adding negative cross-platform checks.
- [ ] Mettre en place des tests négatifs côté serveur : rejeter les preuves avec paramètres corrompus, root obsolète, nullifier non unique, leaf non valide.
- [ ] Documenter le processus de synchronisation des versions et des paramètres entre client et serveur.
- [ ] Sécuriser la base de données des nullifiers et roots côté serveur (intégrité, rollback, suppression malveillante).
- [ ] Protéger les secrets et clés côté client (stockage sécurisé, pas de fuite dans les logs).

### common (library)
- [ ] Stabilize `zkp_prelude` APIs for prover/verifier consumption; add versioned types.
- [ ] Define proof data models (serde) for membership + nullifier; add helpers to compute nullifier from leaf material.
- [ ] Property tests: invalid indices, malformed proofs, even/odd-leaf parity, root mismatch, tampered siblings.

#### ZKP Security & Robustness Additions (from code review)
- [ ] Use official/documented Poseidon parameters (e.g. Filecoin/Zcash) instead of ad-hoc generation; document and verify parameter source.
- [ ] Enforce leaf = Poseidon(secret || scope) in circuit constraints to tightly bind leaf to identity/secret.
- [ ] Strengthen nullifier construction: nullifier = Poseidon(scope || secret || leaf_index) to reduce collision risk and ensure uniqueness per identity/scope.
- [ ] Add circuit constraints to check Merkle path height and leaf_index bounds; validate leaf_index_bits matches leaf_index.
- [ ] Hash scope before use in circuit to ensure entropy and prevent trivial scopes.
- [ ] Integrate a hash (e.g. SHA256) of Poseidon parameters as a public input to proofs, to prevent corrupted parameter attacks.
- [ ] Add vk_version as a public input to proofs to prevent rollback attacks.
- [ ] Add negative tests: reject proofs with invalid Merkle paths, nullifiers, or leaf indices; test for reproducibility across machines.

### blockchain-server (node)
- [x] Expose latest commitment root and recent history (endpoint or via block metadata).
- [x] Anchor root into blocks (store root per block header or body) and make it queryable.
- [x] New tx types: `AnonymousVote`, `AnonymousSupport` (feature-gated); envelope `{ proof, nullifier, scope, payload }` wired.
- [x] Verification pipeline: root acceptance window; nullifier uniqueness (DB + within-block); Groth16 verifier (feature-gated) with VK versioning; persistence of nullifiers.
- [x] Mempool hygiene: filter invalid anonymous txs and de-dupe (scope,nullifier) before mining.
- [ ] Metrics/tracing: proof verification timings, failure reasons; feature flag to disable anon flow.
	- Note: basic tracing/logging is in place; add timings later.
	- Added mock verifier path under `zkp_mock` for deterministic tests.

### wallet-cli (client)
- [ ] Prover integration: fetch latest root, build circuit inputs, generate membership proofs, compute scoped nullifier.
- [ ] UX: retries on root drift (fetch current root, regenerate if necessary), caching/proof re-use where valid.
- [ ] Benchmarks: proof generation time/size on typical hardware; optional settings to trade speed/size.

### tooling/infra
- [ ] Root snapshot/export job (optional signed root attestations for audit/archival).
- [ ] Dev CLIs: rebuild proofs from stored leaves; inspect nullifier sets; validate historical roots.

### CI & security
- [ ] CI matrix: with/without identity feature; run ZKP unit/integration tests.
- [x] Sensitive string grep: ensure no secrets or raw private keys in logs.
- [ ] Threat model doc updates: replay, front-running via nullifiers, issuer set changes, credential expiry.

#### ZKP Security & Robustness Additions (from code review)
- [ ] Add CI checks for Poseidon parameter integrity and versioning.
- [ ] Add CI tests for negative cases (invalid proofs, corrupted params, rollback attempts).

### Documentation
- [ ] ZKP design doc (`docs/zkp_design.md`): circuits, constraints, data flows, performance notes.
- [ ] Operator guide: how node verifies proofs and monitors nullifier DB.
- [ ] E2E guide: anonymous vote/support walkthrough with wallet-cli.

#### ZKP Security & Robustness Additions (from code review)
- [ ] Document Poseidon parameter source and verification process.
- [ ] Document leaf/nullifier linkage to identity and scope, and circuit constraints for input validation.
- [ ] Add section on negative test cases and expected rejection scenarios.

 ## Milestones
- [x] M1 (1 week): Choose ZKP stack; finalize circuit scopes and accumulator policy; create skeleton types. (See `docs/zkp_stack_decision.md`)
- [x] M2 (1–2 weeks): Implement node plumbing (latest roots, tx types, verifier path) + common models + tests. (Includes mock VK-based verifier test)
- [x] M3 (1–2 weeks): Wallet proof generation (membership + nullifier) + initial E2E integration test.
  - ✅ Implemented MembershipNullifierCircuit with MiMC constraints for Merkle path verification and nullifier derivation
  - ✅ Added node API endpoint `/identity/proof/{commitment_hex}` for fetching Merkle inclusion proofs
  - ✅ Updated wallet CLI to use real VC commitments and fetched proofs instead of dummies
  - ✅ Code compiles successfully; ready for E2E testing
  - ✅ API endpoints verified working: identity proof fetching returns valid Merkle proofs with path and directions
  - ✅ Server runs and serves proofs for committed identities
  - ✅ **Integration test passes**: `zkp_verify_endpoint_roundtrip` successfully generates proof, posts to server, and verifies
- [ ] M4 (1–2 weeks): Robustness (revocations, issuer set updates), performance tuning, detailed docs.
  - [x] E2E Integration Test: API verification completed - wallet can fetch real Merkle proofs from node
  - [x] Fix ZKP Circuit Compilation: Resolved ark_r1cs_std import issues in wallet-cli
  - [x] Complete Full E2E Test: Run end-to-end proof generation and verification with working circuit
  - [x] Full automation: end-to-end automation script (`scripts/zkp_flow*.ps1`) ran successfully and verified server-side Groth16 verification on 2025-10-01.
  - [ ] Benchmark Proof Generation: Measure time/size for typical hardware, add optional settings for trade-offs
  - [ ] Robustness Features: Implement root drift retries, proof re-use caching, revocation handling

## Risks and mitigations
- Prover performance on low-end devices: prefer fast systems (Groth16 fallback), allow offline preparation and caching.
- Circuit evolution: version proofs and verifiers; maintain backward-compatible acceptance windows.
- Root drift and replay: accept proofs against recent roots; provide clear wallet retry guidance; bind nullifier to scope.
- Issuer/credential changes: define policy for commitment set updates and their effect on proof validity.
