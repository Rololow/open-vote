# Troubleshooting: Identity (DID/VC)

Common issues and fixes when running the identity flow.

## 1) Invalid signature (tampered VC)
- Symptom: `/issuer/verify` returns `{ valid: false }` with reason `signature_invalid`.
- Fixes:
  - Ensure the VC wasn't modified after issuance (especially `proof` block).
  - Recompute digest only on the unsigned VC (without `proof`).

## 2) Unknown issuer
- Symptom: Commit or submit rejected, node logs mention disallowed issuer.
- Fixes:
  - Set `ALLOWED_ISSUERS_DIDS` env var to include the issuer DID or leave empty in dev.
  - Confirm issuer DID via `issuer_key_tool` output.

## 3) Expired VC
- Symptom: Commit rejected or chain validation fails due to expiry.
- Fixes:
  - Adjust `ISSUER_VC_VALIDITY_DAYS` or request a fresh VC.

## 4) Commitment not found (404)
- Symptom: GET `/api/identity/commitments/<hash>` returns 404 after commit attempt.
- Fixes:
  - Use wallet-cli `vc-commit` which auto-registers missing commitments.
  - Check node logs for constraint violation on duplicate commits (idempotent expected).

## 5) JWK export not found
- Symptom: `/issuer/jwk` not writing a file when `ISSUER_PUBKEY_EXPORT` set.
- Fixes:
  - Ensure the path is writable and the process has permissions.
  - Use `issuer_key_tool --export-jwk` to export offline.

## 6) OneDrive/Spaces path quirks on Windows
- Symptom: quoting issues or file-not-found.
- Fixes:
  - Prefer paths without spaces for scripts. Wrap paths in quotes when required.

See also:
- `docs/identity_diagram.md` for the sequence and data flow
- `docs/identity_hash.md` for canonicalization and digest
- `README.md` section "Parcours E2E Identité (DID/VC)"
