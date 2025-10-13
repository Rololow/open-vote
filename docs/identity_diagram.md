# Identity flow (Mermaid)

```mermaid
sequenceDiagram
    autonumber
    participant W as Wallet (Citizen)
    participant I as Issuer (did:key)
    participant B as Blockchain Node

    Note over W: Generate Ed25519 keypair (local)
    W->>W: did:key derivation

    W->>I: Request credential (subject DID)
    I->>I: Load issuer key, derive issuer DID
    I->>I: Build unsigned VC (issuer, subject, dates, claims)
    I->>I: Canonicalize JSON and sign (Ed25519)
    I-->>W: Return signed VC (proof)

    Note over W: Compute commitment hash on canonicalized unsigned VC
    W->>B: POST /api/identity/commit { commitment_hash, issuer_did, did }
    B->>B: Store idempotently, mark active if not expired and issuer allowed

    W->>B: Submit transaction referencing identity_ref
    B->>B: Validate identity_ref (exists, active, not expired, issuer allowed)
    B-->>W: Include transaction in block upon mining

    W->>I: POST /issuer/verify { credential }
    I->>I: Recompute digest, recover did:key, verify signature
    I-->>W: { valid, commitment_hash?, reason? }
```

---

- Issuer public key export: `/issuer/jwk` (kid is deterministically derived from pubkey)
- Commitment log: append-only `commitments.log` → Merkle root via `compute_root`
