param(
    [string]$NodeUrl = "http://127.0.0.1:3000",
    [string]$IssuerEndpoint = "",
    [string]$Store = "$HOME/.e-gov-wallet/keys.enc",
    [Parameter(Mandatory=$true)][string]$Passphrase,
    [string]$KeyId = "",
    [string]$CredDir = "$HOME/.e-gov-wallet/credentials",
    # If set, the script will start the blockchain-server locally and wait for /health to respond
    [switch]$AutoStartServer = $true
)

$ErrorActionPreference = "Stop"

function Step($msg) { Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Ok($msg) { Write-Host "[OK] $msg" -ForegroundColor Green }
function Warn($msg) { Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red }

# Start blockchain server helper (starts via `cargo run` in repo root)
function Start-BlockchainServer {
    param([string]$RepoRoot)
    Write-Host "[STEP] Starting blockchain-server (background)..." -ForegroundColor Cyan
    $proc = Start-Process -FilePath "cargo" -ArgumentList "run -p blockchain-server --bin blockchain-server" -WorkingDirectory $RepoRoot -NoNewWindow -PassThru
    return $proc
}

# Stop blockchain server helper
function Stop-BlockchainServer {
    param([System.Diagnostics.Process]$Proc)
    if ($null -ne $Proc) {
        Write-Host "[STEP] Stopping blockchain-server (pid $($Proc.Id))" -ForegroundColor Cyan
        try { Stop-Process -Id $Proc.Id -Force } catch { Write-Host "[WARN] Failed to stop process: $($_.Exception.Message)" -ForegroundColor Yellow }
    }
}

# Headless passphrase for wallet-cli (used when --passphrase is not provided)
$env:WALLET_PASSPHRASE = $Passphrase

# Ensure credentials directory exists
if (-not (Test-Path -LiteralPath $CredDir)) { New-Item -ItemType Directory -Force -Path $CredDir | Out-Null }

# Repo root (one level up from scripts)
$RepoRoot = Resolve-Path -Path (Join-Path $PSScriptRoot "..")

# server process tracking
$serverProc = $null
$startedServer = $false

try {

# 1) Keystore init (idempotent)
Step "Initializing keystore at $Store"
try {
    cargo run -q -p wallet-cli --bin wallet-cli -- keystore-init --passphrase $Passphrase --store $Store | Out-Host
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
    $gen = cargo run -q -p wallet-cli --bin wallet-cli -- keystore-generate-keypair --passphrase $Passphrase --store $Store 2>&1
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
$didOut = cargo run -q -p wallet-cli --bin wallet-cli -- did-generate --key-id $KeyId --store $Store 2>&1
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
    # Pre-flight: check issuer /health so we don't fail noisily when it's down
    $issuerOk = $false
    try {
        $h = Invoke-RestMethod -Uri "$IssuerEndpoint/health" -Method GET -TimeoutSec 5
        if ($h) { $issuerOk = $true }
    } catch {
        Warn "Issuer at $IssuerEndpoint appears unreachable: $($_.Exception.Message)"
    }

    # If issuer is down and AutoStartServer requested, start server and wait for /health
    if ((-not $issuerOk) -and $AutoStartServer) {
        Write-Host "[STEP] AutoStartServer enabled: launching blockchain-server and waiting for issuer /health..." -ForegroundColor Cyan
        $serverProc = Start-BlockchainServer -RepoRoot $RepoRoot
        if ($serverProc) { $startedServer = $true }
        # wait loop
        for ($i = 0; $i -lt 60; $i++) {
            try {
                $h = Invoke-RestMethod -Uri "$IssuerEndpoint/health" -Method GET -TimeoutSec 2
                if ($h) { $issuerOk = $true; break }
            } catch {
                Write-Host "[WAIT] issuer not ready (attempt $($i+1))"
                Start-Sleep -Seconds 1
            }
        }
        if (-not $issuerOk) { Warn "Issuer did not become ready after waiting; skipping VC request." }
    }

    if ($issuerOk) {
        Step "Requesting VC from issuer at $IssuerEndpoint"
        cargo run -q -p wallet-cli --bin wallet-cli -- vc-request --endpoint $IssuerEndpoint --subject-did $Did --out-dir $CredDir | Out-Host
        # Find most recent VC file in $CredDir
        $vc = Get-ChildItem -LiteralPath $CredDir -Filter *.json | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($null -eq $vc) { Fail "No VC file found in $CredDir after vc-request"; exit 1 }
        Ok "VC file: $($vc.FullName)"

        Step "Committing VC (public presence) to node at $NodeUrl (if available)"
        try {
            # wallet-cli expects the node API base (the server mounts API under /api).
            # If the provided $NodeUrl does not already include the /api prefix, add it for wallet-cli calls.
            if ($NodeUrl.TrimEnd('/') -match '/api$') { $NodeApiUrl = $NodeUrl.TrimEnd('/') } else { $NodeApiUrl = $NodeUrl.TrimEnd('/') + '/api' }
            Write-Host "[INFO] Using node API URL for vc-commit: $NodeApiUrl"
            cargo run -q -p wallet-cli --bin wallet-cli -- vc-commit --file $vc.FullName --dir $CredDir --node-url $NodeApiUrl --issuer-endpoint $IssuerEndpoint | Out-Host
            Ok "VC commit attempted"
        } catch {
            Warn "vc-commit failed (node or issuer may be unavailable): $($_.Exception.Message)"
        }
    } else {
        Warn "Skipping VC request because issuer is not reachable at $IssuerEndpoint"
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
    cargo run -q -p wallet-cli --bin wallet-cli -- create-proposal --title $title --description $desc --key-id $KeyId --store $Store --node-url $NodeUrl | Out-Host
    Ok "Proposal submitted"
}

    Ok "Headless CI flow completed"
} finally {
    if ($startedServer -and $null -ne $serverProc) {
        Stop-BlockchainServer -Proc $serverProc
    }
}
