# ZKP Stack Decision (Phase 3)

Date: 2025-09-29

This document evaluates candidate zero-knowledge proof stacks for Phase 3 (anonymous-but-verifiable actions) and records the decision and its implications.

## Problem statement
We need to verify, on-node, anonymous actions (e.g., vote/support) using:
- A membership proof that a citizen’s commitment is included in the latest (or recent) Merkle root of identity commitments.
- A scoped nullifier to enforce one-person-one-action per scope without revealing identity.

The wallet generates proofs; the node verifies deterministically and cheaply. The current commitment accumulator and root anchoring are based on SHA-256 and are exposed via `/api/identity/root` and `/api/identity/roots`.

## Requirements and constraints
- Security: sound proofs; collision-resistant nullifier; replay protection; acceptance against recent root set.
- Performance: fast verification on node; proof size reasonable for transaction payloads; prover time acceptable on typical machines.
- Determinism and auditability: clear, versioned formats; deterministic verification; bounded CPU/memory.
- Integration: Rust-first, easy to integrate with existing crates; compatible with SHA-256 Merkle membership (gadgets or strategy to bridge).
- Operational: straightforward build (CI on Windows/Linux), permissive licenses.
- Setup: minimize ceremony burden; if trusted setup is required, keep it simple and documentable.

## Options considered

1) Halo2 (Plonkish, KZG)
- Pros:
  - Modern proving system with good ecosystem traction in Rust.
  - Universal (or updatable) SRS models possible; avoids per-circuit setup.
  - Flexible circuits; recursion paths exist.
- Cons:
  - SHA-256 inside circuits is relatively heavy (vs ZK-friendly hashes like Poseidon).
  - Circuit authoring and tooling are more advanced but also more complex; requires careful engineering.
  - Build/tooling maturity varies across platforms.

2) Arkworks + Groth16 (R1CS)
- Pros:
  - Mature Rust libraries (arkworks) with well-known Groth16 flow.
  - Small proofs and fast verification; widely used and well understood.
  - SHA-256 gadgets exist (heavier than ZK-friendly hashes but feasible for membership path depths typical of our use-case).
- Cons:
  - Requires a per-circuit trusted setup (Phase 2 of Powers of Tau), which needs to be managed and documented.
  - Circuit changes imply re-running setup and changing verifying keys.

3) Plonky2
- Pros: very fast prover, recursion-friendly.
- Cons: larger proofs and heavier verification; ecosystem integration for our immediate needs is less direct; added complexity.

4) zkVMs (e.g., RISC Zero)
- Pros: general-purpose proving of arbitrary programs; developer friendly for some workflows.
- Cons: proof sizes and verification costs are not aligned with our small, purpose-built circuits; unnecessary complexity for our scope.

5) Noir/Barretenberg (and similar)
- Pros: ergonomic DSL; fast prover in some setups.
- Cons: mixed language/toolchain overhead (C++), integration and CI complexity; adds another stack beyond our Rust workspace.

## Decision
Adopt a two-track strategy:
- MVP path (immediate): implement membership + nullifier circuits using Arkworks + Groth16.
  - Rationale: fast-to-integrate Rust stack, small proofs, fast verification, and well-understood tooling; suitable for Phase 3 MVP.
  - Manage the trusted setup with a documented, minimal ceremony (Phase 1 Powers of Tau + per-circuit Phase 2), and publish verifying keys.
- Strategic path (future-ready): design the verifier/prover APIs behind a thin compatibility layer so we can prototype a Halo2 implementation in parallel and optionally migrate/dual-accept later.
  - Keep proof payloads versioned and include a `scheme`/`vk_version` field to allow the node to accept a compatible set.

This balances time-to-value and long-term maintainability without locking us into a single system.

## Architecture implications
- Proof model: `ProofEnvelope { scheme, vk_version, root, scope, nullifier, proof_bytes, public_inputs }` (serde), where `scheme` distinguishes `groth16` vs future `halo2`.
- Verifier trait in `common` (or `blockchain-server`):
  - `verify_membership_and_nullifier(root, scope, nullifier, proof_envelope) -> bool`
  - Implementations: `groth16::Verifier` (MVP) and later `halo2::Verifier`.
- Nullifier scheme: `nullifier = H(scope || secret)` with domain separation; choose a ZK-friendly hash inside the circuit OR use a SHA-256 gadget to align with existing infrastructure. For MVP, we will keep SHA-256 for membership path and may use Poseidon for nullifier (documented and domain-separated), with an on-chain verification that only checks the circuit’s constraints.
- Root acceptance window: node accepts proofs against the latest root and N most recent anchored roots. `/api/identity/roots` is already available to wallets.
- Storage: DB table for nullifiers `(scope, nullifier, tx_id, timestamp)` with a unique index on `(scope, nullifier)` to reject duplicates deterministically.

## Trusted setup notes (Groth16)
- Use community Powers of Tau Phase 1 or run a minimal internal ceremony to derive Phase 2 per-circuit keys.
- Version verifying keys and publish their fingerprints; bundle verifying keys into the node binary or load from a signed file.
- Changing circuits implies bumping `vk_version` and running a new setup; keep acceptance windows and migration guides.

## Performance considerations
- Membership proofs require hashing along the sibling path. With SHA-256 gadgets, the cost scales with Merkle depth (~log2(leaves)). Keep commitment sets curated (deduplicate/revoke) and accept proofs against recent roots to reduce wallet churn, though depth is a function of set size.
- Verification cost should remain small on-node; Groth16 verification generally performs well for concise public inputs.
- Prover time is borne by wallets/clients; provide clear UX and timeouts.

## Risks and mitigations
- Trusted setup burden: mitigate with documented ceremony and strong key/versioning hygiene.
- Circuit evolution: version payloads and verifying keys; dual-accept during migrations.
- Hash mismatch: SHA-256 is not ZK-friendly; initial cost is acceptable for MVP. Consider a Poseidon-based accumulator in a future iteration, with a well-defined bridge strategy.
- Tooling/CI complexity: keep dependencies minimal; gate with a cargo feature (`zkp`) and add CI jobs accordingly.

## Next steps (execution plan)
1) Define proof payload types and traits in `common` (feature-gated `zkp`).
2) Implement Groth16 circuits:
  - Membership over SHA-256 Merkle path.
  - Scoped nullifier computation (domain-separated).
  - Public inputs: root, scope, nullifier; private inputs: leaf secret(s), path.
3) Add verifier to node:
  - Load verifying keys (bundled or from file).
  - New tx types: `AnonymousVote`, `AnonymousSupport` carrying proof envelope.
  - Verification pipeline with nullifier uniqueness enforced in DB.
4) Wallet CLI:
  - Fetch latest root; build inputs; generate proof; submit tx.
5) Tests & benches:
  - Unit/integration tests (happy path + invalid proof/root/duplicate nullifier).
  - Bench verifier timings; CI job with/without `identity`/`zkp` features.

## Appendix: alternative (Halo2) track
- Keep the verifier/prover traits and ProofEnvelope stable.
- Prototype a Halo2 circuit performing the same constraints.
- Add an optional acceptance mode to verify either `groth16` or `halo2` proofs based on `scheme` and `vk_version`.