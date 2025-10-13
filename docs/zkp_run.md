## ZKP end-to-end run (operator guide)

This short guide explains how to run the repository's end-to-end ZKP automation (wallet -> proof -> server verification) and how to inspect the verification logs (`errors.log`). It's intended for operators and CI authors.

### Purpose

- Run the automated end-to-end flow that builds the wallet and server, generates a Groth16 proof with the wallet CLI, POSTs the proof to the node, and verifies it server-side.
- Make it easy to inspect verification diagnostics (`errors.log`) to validate Poseidon params and VK synchronization and to debug verification failures.

### Prerequisites (host running the script)

- Windows / PowerShell (the scripts are PowerShell; Linux/macOS can use `pwsh` if PowerShell Core is installed).
- Rust toolchain (cargo) and any native libs required by your toolchain.
- `pwsh` available on PATH (or PowerShell on Windows).
- Optionally: Docker if you prefer containerized runs (not required by the script shipped with this repo).

### Quick run (local)

From the repository root run in PowerShell (this is the script used during development):

```powershell
# runs the orchestration that builds, starts the server, runs the wallet CLI, generates a proof and posts it
pwsh -File .\scripts\zkp_flow_with_clean_log.ps1
```

- The script will create a temporary data directory (printed in its output) and will stop and clean up the server after the run completes.
- On success the script prints a completion message and the location of temporary files created during the run.

### Quiet / CI-friendly run

Use the same command in a CI job. The script exits with a non-zero code on failure. If you need to capture the generated artifacts, set the environment variable `BLOCKCHAIN_DATA_DIRECTORY` to a persistent path before running the script so artifacts and keys are kept.

Example (PowerShell):

```powershell
$env:BLOCKCHAIN_DATA_DIRECTORY = 'C:\ci-artifacts\zkp-data'
pwsh -File .\scripts\zkp_flow_with_clean_log.ps1
```

### Artifacts the script produces (temporary by default)

- Proof envelope (JSON): `<tmp>/zkp_proof.json`  — contains the proof bytes, public inputs, vk_version and metadata.
- Proof binary: `<tmp>/zkp_proof.bin` — raw serialized Groth16 proof bytes.
- VK and PK files under `<tmp>/zkp/` (wallet writes them to the data dir). Example: `vk-groth16-v1.bin`, `pk-groth16-v1.bin`.
- Poseidon params file: `<tmp>/zkp/poseidon_params.bin` (if produced by the wallet CLI).

If you set `BLOCKCHAIN_DATA_DIRECTORY` to a fixed path the script will leave these files in that folder so they can be archived by CI.

### Inspecting verification logs (`errors.log`)

The verification step writes helpful diagnostic lines to `errors.log` in the repository root. Key lines to look for (examples):

- `Poseidon params hash (serveur): <hex>` and `Poseidon params hash (client): <hex>` — these must match. If they differ the server and client used different Poseidon parameters.
- `VK hash (serveur): <hex>` and `VK hash (cli): <hex>` — verifying key mismatch indicates the server loaded a different VK file than the wallet used to generate the proof.
- `Proof deserialization error: ...` — indicates the server could not parse the posted proof bytes.
- `[verify_zkp_handler] Groth16 verification result: true|false` — final verification outcome.

PowerShell commands to view relevant lines quickly:

```powershell
# Tail the log live
Get-Content .\errors.log -Wait -Tail 50

# Show only Poseidon/VK/verify lines
Select-String -Path .\errors.log -Pattern "Poseidon params hash|VK hash|Groth16 verification" -SimpleMatch
```

If `Groth16 verification result` is `false`, inspect the preceding lines to check for:

- mismatched Poseidon/VK hashes
- proof deserialization errors
- public inputs printed in the log — ensure public input ordering/encoding matches the wallet's export

### Troubleshooting checklist

1. Confirm the wallet and server used the same `BLOCKCHAIN_DATA_DIRECTORY` (the script sets this by default for the ephemeral run). If you ran the server separately, ensure it pointed to the same data dir.
2. Confirm `poseidon_params.bin` exists in the same data dir and was written by the wallet.
3. Compare file hashes (VK and poseidon params) between wallet's files and server's loaded files (use `Get-FileHash` in PowerShell).
4. If proof deserialization fails, re-check the encoding the wallet used (compressed vs uncompressed ark-serialize). The repository uses canonical serialize/deserialize and the scripts expect that format.

Example: PK/VK/poseidon hash check (PowerShell)

```powershell
Get-FileHash .\data\zkp\vk-groth16-v1.bin -Algorithm SHA256
Get-FileHash .\data\zkp\poseidon_params.bin -Algorithm SHA256
```

### CI recommendations

- Use a dedicated ephemeral working directory per CI job (e.g. set `BLOCKCHAIN_DATA_DIRECTORY`) to avoid accidental reuse of local artifacts.
- Record `errors.log` as a job artifact if the script fails; it contains the most important diagnostics.
- Add a smoke test that runs `pwsh -File .\scripts\zkp_flow_with_clean_log.ps1` with a time budget (e.g. 5 minutes) and fails the build if exit code != 0.

### When to escalate

- If Poseidon/VK hashes match but `Groth16 verification result: false`, collect:
  - the `errors.log` lines around the verification (proof bytes, public inputs, VK hash)
  - the wallet-side verification logs (wallet prints a `direct verify = true` line when local verification succeeds)
  - a copy of the posted proof bytes and public inputs

- Attach those to an issue for deeper cryptographic debugging (field encodings, public input ordering, curve mismatches).

---

If you want, I can also add a small CI job template (GitHub Actions) that runs the script and archives `errors.log` and the produced artifacts when failures occur. Would you like that next?
