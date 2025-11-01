#!/usr/bin/env bash
set -euo pipefail

# Defaults
NODE_URL="http://127.0.0.1:3000"
ISSUER_ENDPOINT=""
STORE="$HOME/.e-gov-wallet/keys.enc"
PASSPHRASE=""
KEY_ID=""
CRED_DIR="$HOME/.e-gov-wallet/credentials"

usage() {
  cat <<'EOF'
Usage: ci_wallet_headless.sh [--node-url URL] [--issuer URL] [--store PATH] --passphrase PASS [--key-id ID] [--cred-dir DIR]

Environment variables (alternative to flags):
  NODE_URL, ISSUER_ENDPOINT, STORE, PASSPHRASE, KEY_ID, CRED_DIR
EOF
}

# Parse flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    --node-url) NODE_URL="$2"; shift 2;;
    --issuer) ISSUER_ENDPOINT="$2"; shift 2;;
    --store) STORE="$2"; shift 2;;
    --passphrase) PASSPHRASE="$2"; shift 2;;
    --key-id) KEY_ID="$2"; shift 2;;
    --cred-dir) CRED_DIR="$2"; shift 2;;
    -h|--help) usage; exit 0;;
    *) echo "Unknown arg: $1"; usage; exit 1;;
  esac
done

# Env fallback
NODE_URL="${NODE_URL:-${NODE_URL}}"
ISSUER_ENDPOINT="${ISSUER_ENDPOINT:-${ISSUER_ENDPOINT:-}}"
STORE="${STORE:-${STORE}}"
PASSPHRASE="${PASSPHRASE:-${PASSPHRASE:-}}"
KEY_ID="${KEY_ID:-${KEY_ID:-}}"
CRED_DIR="${CRED_DIR:-${CRED_DIR}}"

if [[ -z "$PASSPHRASE" ]]; then
  echo "ERROR: --passphrase or PASSPHRASE env is required" >&2
  exit 1
fi

step() { printf "[STEP] %s\n" "$*"; }
ok() { printf "[OK] %s\n" "$*"; }
warn() { printf "[WARN] %s\n" "$*"; }
fail() { printf "[FAIL] %s\n" "$*"; }

# Headless mode
export WALLET_PASSPHRASE="$PASSPHRASE"

mkdir -p "$CRED_DIR"

# 1) Keystore init (idempotent)
step "Initializing keystore at $STORE"
if cargo run -q -p wallet-cli --bin wallet-cli -- keystore-init --passphrase "$PASSPHRASE" --store "$STORE"; then
  ok "Keystore initialized"
else
  warn "Keystore may already exist at $STORE"
fi

# 2) Generate/import key if not provided
if [[ -z "$KEY_ID" ]]; then
  step "Generating and importing keypair into keystore"
  OUT=$(cargo run -q -p wallet-cli --bin wallet-cli -- keystore-generate-keypair --passphrase "$PASSPHRASE" --store "$STORE" 2>&1 || true)
  printf "%s\n" "$OUT"
  KEY_ID=$(printf "%s\n" "$OUT" | grep -o 'id=[^ ]\+' | head -1 | cut -d= -f2 || true)
  if [[ -z "$KEY_ID" ]]; then
    fail "Unable to parse generated key id from output"; exit 1
  fi
  ok "Key generated: id=$KEY_ID"
else
  step "Using provided key id: $KEY_ID"
fi

# 3) Derive did:key from keystore key
step "Deriving did:key from key-id=$KEY_ID"
DID_OUT=$(cargo run -q -p wallet-cli --bin wallet-cli -- did-generate --key-id "$KEY_ID" --store "$STORE" 2>&1)
printf "%s\n" "$DID_OUT"
DID=$(printf "%s\n" "$DID_OUT" | grep -o 'did:key:[^ ]\+' | head -1 || true)
if [[ -z "$DID" ]]; then
  fail "Unable to parse did:key from output"; exit 1
fi
ok "DID: $DID"

# 4) Optional VC request/commit
if [[ -n "$ISSUER_ENDPOINT" ]]; then
  step "Requesting VC from issuer at $ISSUER_ENDPOINT"
  cargo run -q -p wallet-cli --bin wallet-cli -- vc-request --endpoint "$ISSUER_ENDPOINT" --subject-did "$DID" --out-dir "$CRED_DIR"
  VC_FILE=$(ls -1t "$CRED_DIR"/*.json 2>/dev/null | head -1 || true)
  if [[ -z "$VC_FILE" ]]; then
    fail "No VC file found in $CRED_DIR after vc-request"; exit 1
  fi
  ok "VC file: $VC_FILE"
  step "Committing VC to node (public presence)"
  if ! cargo run -q -p wallet-cli --bin wallet-cli -- vc-commit --file "$VC_FILE" --dir "$CRED_DIR" --node-url "$NODE_URL" --issuer-endpoint "$ISSUER_ENDPOINT"; then
    warn "vc-commit failed (node or issuer may be unavailable)"
  fi
fi

# 5) Create a proposal signed via keystore if node is up
if curl -fsS "$NODE_URL/health" >/dev/null 2>&1; then
  step "Creating a proposal (signed with key-id=$KEY_ID)"
  TITLE="CI Proposal $(date -Is)"; DESC="Automated run $(date -Is)"
  cargo run -q -p wallet-cli --bin wallet-cli -- create-proposal --title "$TITLE" --description "$DESC" --key-id "$KEY_ID" --store "$STORE" --node-url "$NODE_URL"
  ok "Proposal submitted"
else
  warn "Node health check failed at $NODE_URL/health. Skipping proposal submission."
fi

ok "Headless CI flow completed"
