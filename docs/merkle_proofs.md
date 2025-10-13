# Merkle proofs (Phase 3 groundwork)

This project prepares anonymous membership proofs using a simple Merkle accumulator. You can:

- Compute the Merkle root over identity commitments (append-only log)
- Generate a Merkle proof for a leaf index
- Verify a proof against the current root

The implementation lives in `common::identity::zkp_prelude` and is intentionally minimal (no heavy ZKP deps yet).

## File format: `commitments.log`

- One 32-byte commitment per line, as lowercase hex (64 chars)
- No prefixes, no JSON – just raw hex, newline-delimited

Example (truncated):

```
8d969eef6ecad3c29a3a629280e686cf0c3f5d5a86aff3ca12020c923adc6c92
9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
2e7d2c03a9507ae265ecf5b5356885a53393a2029d241394997265a1a25aefc6
```

## CLI: `commitment_proof`

- Compute root: prints `root=<hex>`
- Generate proof: prints a JSON with `leaf_index` and `siblings: [hex...]`
- Verify proof: prints `valid=true/false` and exits non-zero if invalid

Examples (PowerShell):

- Compute root from default log:
  - `cargo run -p blockchain-server --bin commitment_proof -- root`
- Compute root from a specific file:
  - `cargo run -p blockchain-server --bin commitment_proof -- ./blockchain-server/data/commitments.log root`
- Generate proof for leaf index 3 and save it:
  - `cargo run -p blockchain-server --bin commitment_proof -- ./blockchain-server/data/commitments.log prove 3 > proof.json`
- Verify a proof:
  - `cargo run -p blockchain-server --bin commitment_proof -- ./blockchain-server/data/commitments.log verify 3 ./proof.json`

Notes:
- The root/proof derivation mirrors `MerkleAccumulator` in `common` and duplicates the last node when the level has an odd count.
- The CLI is a developer tool for debugging and manual validation; applications should use the library functions directly.

## Library API (Rust)

From `common::identity::zkp_prelude`:
- `MerkleAccumulator`: append leaves and compute `root()`
- `merkle_proof_for(leaves, index) -> MerkleProof`
- `verify_merkle_proof(leaf, &proof, &root) -> bool`

There are unit tests covering even/odd trees and proof roundtrips; an extra integration test checks hex parsing compatibility.