param(
    [string]$NodeUrl = "http://127.0.0.1:3000",
    [string]$IssuerEndpoint = "",
    [string]$Store = "$HOME/.e-gov-wallet/keys.enc",
    [Parameter(Mandatory=$true)][string]$Passphrase,
    [string]$KeyId = "",
    [string]$CredDir = "$HOME/.e-gov-wallet/credentials"
)

$ErrorActionPreference = "Stop"

function Step($msg) { Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Ok($msg) { Write-Host "[OK] $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red }

# Headless passphrase for wallet-cli (used when --passphrase is not provided)
$env:WALLET_PASSPHRASE = $Passphrase

# Ensure credentials directory exists
if (-not (Test-Path -LiteralPath $CredDir)) { New-Item -ItemType Directory -Force -Path $CredDir | Out-Null }

# 1) Keystore init (idempotent)
Step "Initializing keystore at $Store"
try {
    cargo run -p wallet-cli -- keystore-init --passphrase $Passphrase --store $Store | Out-Host
    Ok "Keystore initialized"
} catch {
    if ($_.Exception.Message -match "existe déjà") {
        Warn "Keystore already exists: $Store"
    } else {
        throw $_
    }
}

# 2) Generate/import key (if no KeyId provided)
if ([string]::IsNullOrWhiteSpace($KeyId)) {
    Step "Generating and importing keypair into keystore"
    $gen = cargo run -p wallet-cli -- keystore-generate-keypair --passphrase $Passphrase --store $Store 2>&1
    $gen | Out-Host
    $m = $gen | Select-String -Pattern "id=(?<id>\S+)" -AllMatches | Select-Object -First 1
    if ($m -and $m.Matches.Count -gt 0) {
        $KeyId = $m.Matches[0].Groups['id'].Value
        Ok "Key generated: id=$KeyId"
    } else {
        Fail "Unable to parse generated key id from output"
        exit 1
    }
} else {
    Step "Using provided key id: $KeyId"
}

# 3) Derive did:key from keystore key
Step "Deriving did:key from key-id=$KeyId"
$didOut = cargo run -p wallet-cli -- did-generate --key-id $KeyId --store $Store 2>&1
$didOut | Out-Host
$didMatch = $didOut | Select-String -Pattern "did:key\s*=\s*(?<did>did:key:[^\s]+)" | Select-Object -First 1
if (-not $didMatch) {
    Fail "Unable to parse did:key from did-generate output"
    exit 1
}
$Did = $didMatch.Matches[0].Groups['did'].Value
Ok "DID: $Did"

# 4) Optional VC request/commit if IssuerEndpoint provided
if (-not [string]::IsNullOrWhiteSpace($IssuerEndpoint)) {
    Step "Requesting VC from issuer at $IssuerEndpoint"
    cargo run -p wallet-cli -- vc-request --endpoint $IssuerEndpoint --subject-did $Did --out-dir $CredDir | Out-Host
    # Find most recent VC file in $CredDir
    $vc = Get-ChildItem -LiteralPath $CredDir -Filter *.json | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($null -eq $vc) { Fail "No VC file found in $CredDir after vc-request"; exit 1 }
    Ok "VC file: $($vc.FullName)"

    Step "Committing VC (public presence) to node at $NodeUrl (if available)"
    try {
        cargo run -p wallet-cli -- vc-commit --file $vc.FullName --dir $CredDir --node-url $NodeUrl --issuer-endpoint $IssuerEndpoint | Out-Host
        Ok "VC commit attempted"
    } catch {
        Warn "vc-commit failed (node or issuer may be unavailable): $($_.Exception.Message)"
    }
}

# 5) Create a proposal signed via keystore (if node is up)
$healthOk = $false
try {
    $health = Invoke-RestMethod -Uri "$NodeUrl/health" -Method GET -TimeoutSec 5
    $healthOk = $true
} catch {
    Warn "Node health check failed at $NodeUrl/health. Skipping proposal submission."
}

if ($healthOk) {
    Step "Creating a proposal (signed with key-id=$KeyId)"
    $title = "CI Proposal $(Get-Date -Format s)"
    $desc = "Automated run $(Get-Date -Format s)"
    cargo run -p wallet-cli -- create-proposal --title $title --description $desc --key-id $KeyId --store $Store --node-url $NodeUrl | Out-Host
    Ok "Proposal submitted"
}

Ok "Headless CI flow completed"
