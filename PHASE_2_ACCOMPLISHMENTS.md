# Phase 2 — Accomplishments

Date: 2025-09-29

This document captures what was planned for Phase 2 (identity foundations with DID/VC and commitments), what was actually delivered, where we deviated, and how it was validated.

## Executive summary

- Objective: Introduce issuer-driven identity based on DID/VC, compute pseudonymous commitments for identities, and lay pre-ZKP groundwork (Merkle accumulator, proofs, and tooling). Ensure wallet-led flows and keep private keys off servers.
- Outcome: Delivered an issuer embedded in `blockchain-server` with VC issue/verify endpoints, an identity commitment pipeline persisted in DB and an append-only commitments.log, a minimal Merkle proof system with two CLIs and a shared library, wallet-side helpers, and a comprehensive test/bench/docs set. All workspace tests pass as of 2025-09-29.

## Delivered artifacts

### Issuer endpoints (blockchain-server)
- GET `/issuer/jwk`
  - Exports public JWK for the issuer with deterministic `kid`.
  - Optional file export when `ISSUER_PUBKEY_EXPORT` is set.
  - Source: `blockchain-server/src/api/issuer_api.rs`
- POST `/issuer/credential`
  - Issues a simplified VC (JSON), signs with Ed25519; deterministic canonicalization during signing.
  - Source: `blockchain-server/src/api/issuer_api.rs`
- POST `/issuer/verify`
  - Verifies signature, canonical digest if provided, checks verificationMethod linkage to issuer DID, enforces allowlist `ALLOWED_ISSUERS_DIDS`, and returns `commitment_hash` for convenience.
  - Source: `blockchain-server/src/api/issuer_api.rs`

### Identity commitment pipeline (node)
- Idempotent DB insert for credentials with status=active, issuer/subject DID, issuance/expiration.
- Startup hydration of active commitments; backfills `commitments.log` if missing.
- Validation on transaction submission and pre-mining via `identity_ref`: must exist, be active, not expired, and be issued by an allowed issuer.
- Sources:
  - `blockchain-server/src/node.rs` (hydrate, validate_identity_ref, submit/mining filters)
  - `blockchain-server/src/config.rs` (env config, including `ALLOWED_ISSUERS_DIDS`, `BLOCKCHAIN_DATA_DIR`)

### Pre‑ZKP Merkle layer
- commitments.log
  - Append-only file with one 32-byte SHA-256 hex per line (commitment).
  - Runtime location: `${BLOCKCHAIN_DATA_DIR}/commitments.log` (defaults to `./data/commitments.log`).
- CLI tooling (binaries under `blockchain-server`):
  - `compute_root`: compute Merkle root from `commitments.log`.
    - Source: `blockchain-server/src/bin/compute_root.rs`
  - `commitment_proof`: compute root/proofs/verify.
    - Modes: `root`, `prove <index>`, `verify <index> <proof.json>`
    - Source: `blockchain-server/src/bin/commitment_proof.rs`
- Library APIs (feature-gated under `common`):
  - `common::identity::zkp_prelude`: `IdentityAccumulator`, `MerkleAccumulator`, `MerkleProof`, `merkle_proof_for`, `verify_merkle_proof`.

### Wallet and issuer key tooling
- `issuer_key_tool` (binary; `blockchain-server/src/bin/issuer_key_tool.rs`)
  - Generate/import/export Ed25519 issuer key; deterministic `kid`; derive DID `did:key`.
  - Integration-tested import and DID matching.
- `wallet-cli`
  - Minimal VC flows: request, compute commitment hash, optional `/issuer/verify`, and commit to node.
  - Sources: `wallet-cli/src`.

### Tests, benchmarks, and documentation
- Tests
  - Issuer verify API (positive/negative) and identity commit/mine behavior.
  - Privacy guard tests (no secrets in logs; no server-side private key storage).
  - Integration test for Merkle proofs (hex parsing, proof gen/verify).
  - Sources: `blockchain-server/tests`, `tests/tests/*`.
- Benchmark
  - Criterion-based VC verification micro-bench (Part 7.7) in workspace benches.
- Documentation
  - `docs/identity_diagram.md` — identity flow diagram.
  - `docs/merkle_proofs.md` — Merkle root/proofs CLI and library API.
  - `docs/troubleshooting_identity.md` — identity troubleshooting.
  - README cross-links to Merkle docs and tools.

### Environment variables
- `ALLOWED_ISSUERS_DIDS` — CSV list of allowed issuer DIDs; empty allows all (dev).
- `ISSUER_PUBKEY_EXPORT` — path to export the public JWK emitted by `/issuer/jwk`.
- `BLOCKCHAIN_DATA_DIR` — data directory (contains `commitments.log`); defaults to `./data`.
- `MIGRATIONS_DIR` — used in tests to point at migrations.

## Delta vs original plan
- The “Government Server” is embedded into `blockchain-server` for now to reduce moving parts. The API Gateway’s evolution into a read-only indexer remains planned but is not required for current identity flows.
- DID/VC format is purpose-fit and minimal; adoption of a full W3C VC/DID library (e.g., `ssi`) is deferred to next phase.

## Validation snapshot

- Automated tests: workspace tests are green as of 2025-09-29.
  - Command used:
    - `cargo test --workspace`

- Manual smoke (current commitments.log):
  - Compute root:
    - `cargo run -p blockchain-server --bin compute_root -- ./data/commitments.log`
  - Prove/verify index 0 (adjust index as needed):
    - `cargo run -p blockchain-server --bin commitment_proof -- ./data/commitments.log prove 0 > proof.json`
    - `cargo run -p blockchain-server --bin commitment_proof -- ./data/commitments.log verify 0 proof.json`

## What’s next (rolled into Phase 3)
- Separate government-server process (optional) and API Gateway as an indexer-only service.
- Anonymous actions via ZKP: membership proofs against commitment root(s), nullifier-based one-person-one-action, on-node verification.
- Formalize DID/VC handling with a standard library and expand issuer registry mechanics.
