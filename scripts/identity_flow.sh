#!/usr/bin/env bash
set -euo pipefail

# Minimal Linux CI parity script for identity/key flow and tests.
# Options:
#   --no-cargo        Use prebuilt binaries (skip cargo build)
#   --key <path>      Path to issuer key JSON (default: issuer_ed25519_key.json)
#   --export-jwk <p>  Export public JWK to this path (optional)

NO_CARGO=0
KEY_PATH="issuer_ed25519_key.json"
EXPORT_JWK=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-cargo) NO_CARGO=1; shift;;
    --key) KEY_PATH="$2"; shift 2;;
    --export-jwk) EXPORT_JWK="$2"; shift 2;;
    -h|--help)
      echo "Usage: $0 [--no-cargo] [--key <path>] [--export-jwk <path>]"; exit 0;;
    *) echo "Unknown arg: $1"; exit 2;;
  esac
done

workspace_root="$(cd "$(dirname "$0")/.." && pwd)"
pushd "$workspace_root" >/dev/null

if [[ $NO_CARGO -eq 0 ]]; then
  echo "[build] Building issuer_key_tool and running tests..."
  cargo build -p blockchain-server --bin issuer_key_tool
fi

# Ensure an issuer key exists or create one
echo "[key] Ensuring issuer key at $KEY_PATH"
if [[ -n "$EXPORT_JWK" ]]; then
  cargo run -p blockchain-server --bin issuer_key_tool -- --key "$KEY_PATH" --export-jwk "$EXPORT_JWK"
else
  cargo run -p blockchain-server --bin issuer_key_tool -- --key "$KEY_PATH"
fi

echo "[test] Running workspace tests"
cargo test --workspace

popd >/dev/null
echo "[done] identity_flow.sh completed successfully"
