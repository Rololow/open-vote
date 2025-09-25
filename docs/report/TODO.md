# Report TODO (LaTeX)

This file tracks what remains to finish the report in `main.tex`.

## High priority
- Add an architecture diagram (logical components + data flows). Consider mermaid (for the repo) and a rendered PNG/PDF for LaTeX.
- Add a sequence diagram for VC issuance and verification (subject, issuer, verifier).
- Produce an API reference appendix for major endpoints (request/response schemas).
- Include schema excerpts for core tables (blocks, transactions, accounts, votes, identity_commitments).
- Document the migration loader details (file naming, transactional semantics, failure recovery).

## Medium priority
- Expand the Security chapter: replay protection, mempool checks, consensus assumptions.
- Performance considerations: expected throughput, PoW settings for dev/test, knobs for production.
- Operational runbook: how to run `--migrate`, start services, rotate issuer keys, backups.
- Frontend section: screenshots of the Trunk/Yew interface and notable components.

## Low priority
- Add citations and references (DID Core, Data Integrity 2020, Ed25519 specs).
- Add a glossary of terms (DID, VC, PoW, UTXO, etc.).
- Clean up warnings in the codebase and reflect any changes in the report.

## Build instructions (local)
- Use your LaTeX toolchain of choice (e.g., TeX Live or MiKTeX). Recommended: `latexmk` with `pdflatex` or `xelatex`.
- Suggested command:
  - `latexmk -pdf -interaction=nonstopmode -halt-on-error main.tex`
- If you don’t have images yet, you can compile as-is; placeholders won’t block the build.

## Sources to cross-check
- `blockchain-server/src/api` for endpoints and handlers.
- `blockchain-server/src/issuer` and `common/src/identity/*` for DID/VC details.
- `blockchain-server/src/storage.rs` and `migrations/*.sql` for DB bits.
- `common/src/transaction.rs` and `blockchain-server/src/security.rs` for security rules.
