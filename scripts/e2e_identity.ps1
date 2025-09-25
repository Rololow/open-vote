Param(
  [int]$ApiPort = 8085,
  [string]$BindAddress = "127.0.0.1",
  [switch]$Build = $true,
  [ValidateSet('None','DisallowedIssuer','InvalidSignature')]
  [string]$NegativeMode = 'None'
)

$ErrorActionPreference = 'Stop'

function Write-Step($msg) { Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Write-Ok($msg) { Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Err($msg) { Write-Host "[ERR] $msg" -ForegroundColor Red }

# Paths
$workspace = Split-Path -Parent $PSCommandPath
$root = Split-Path -Parent $workspace
$keyFile = Join-Path $workspace "wallet_key.hex"
$credDir = Join-Path $env:TEMP ("e2e-cred-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $credDir | Out-Null

# Binaries
function Find-Bin([string]$name){
  $p1 = Join-Path $root ("target\\debug")
  $p2 = Join-Path $root ("target\\x86_64-pc-windows-msvc\\debug")
  $candidates = @(
    (Join-Path $p1 $name),
    (Join-Path $p2 $name)
  )
  foreach ($c in $candidates) { if (Test-Path -LiteralPath $c) { return $c } }
  return $null
}

$walletExe = Find-Bin "wallet-cli.exe"
$serverExe = Find-Bin "blockchain-server.exe"

if ($Build) {
  Write-Step "Building server and wallet-cli (debug)…"
  cargo build -p blockchain-server -p wallet-cli | Write-Host
  Write-Ok "Build complete"
}

if (-not $walletExe) { throw "wallet-cli binary not found in target paths" }
if (-not $serverExe) { throw "blockchain-server binary not found in target paths" }

$base = "http://${BindAddress}:${ApiPort}"

# Start server with env vars
Write-Step "Starting blockchain-server on $base"
$env:BLOCKCHAIN_BIND_ADDRESS = $BindAddress
$env:BLOCKCHAIN_API_PORT = "$ApiPort"
# Configure issuer allowlist per mode
switch ($NegativeMode) {
  'DisallowedIssuer' {
    # Set a DID that certainly won't match the server's issuer DID
    $env:ALLOWED_ISSUERS_DIDS = "did:key:zDisallowedIssuer"
    Write-Warn "NegativeMode=DisallowedIssuer: ALLOWED_ISSUERS_DIDS set to $($env:ALLOWED_ISSUERS_DIDS)"
  }
  Default {
    # allow all for the positive path or other modes
    $env:ALLOWED_ISSUERS_DIDS = ""
  }
}

$server = Start-Process -FilePath $serverExe -WorkingDirectory $root -PassThru -WindowStyle Hidden
Start-Sleep -Milliseconds 200

try {
  # Wait for health
  Write-Step "Waiting for /health…"
  $deadline = (Get-Date).AddSeconds(10)
  do {
    try {
      $r = Invoke-WebRequest -Uri ("$base/health") -Method GET -UseBasicParsing -TimeoutSec 2
      if ($r.StatusCode -eq 200) { break }
    } catch { Start-Sleep -Milliseconds 200 }
  } while ((Get-Date) -lt $deadline)
  $r = Invoke-WebRequest -Uri ("$base/health") -Method GET -UseBasicParsing -TimeoutSec 2
  if ($r.StatusCode -ne 200) { throw "API not healthy" }
  Write-Ok "/health OK"

  # Generate wallet keypair
  Write-Step "Generating wallet keypair → $keyFile"
  & $walletExe generate-keypair --output $keyFile | Write-Host

  # Derive did:key
  Write-Step "Deriving did:key from $keyFile"
  $didOut = & $walletExe did-generate --key-file $keyFile
  $didMatch = ($didOut | Select-String -Pattern 'did:key:z\S+' | Select-Object -First 1)
  if (-not $didMatch) { throw "did:key not found in output" }
  $subjectDid = ([regex]::Match($didMatch.Line, 'did:key:z\S+')).Value
  Write-Ok "subject DID: $subjectDid"

  # Request VC
  Write-Step "Requesting VC from issuer at $base"
  & $walletExe vc-request --endpoint $base --subject-did $subjectDid --out-dir $credDir | Write-Host
  $creds = @(Get-ChildItem -Path $credDir -Filter "*.json")
  if ($creds.Count -lt 1) { throw "Expected at least 1 credential file in $credDir, found 0" }
  $credFile = $creds[0].FullName
  Write-Ok "VC saved: $credFile"

  # Compute hash (from filename)
  $hash = [System.IO.Path]::GetFileNameWithoutExtension($credFile)
  Write-Ok "commitment hash: $hash"

  if ($NegativeMode -eq 'InvalidSignature') {
    Write-Warn "Tampering VC to invalidate signature (NegativeMode=InvalidSignature)"
    $vcJson = Get-Content -LiteralPath $credFile -Raw | ConvertFrom-Json
    if ($vcJson.proof -and $vcJson.proof.proofValue) {
      $vcJson.proof.proofValue = ($vcJson.proof.proofValue + 'A')
    } elseif ($vcJson.credentialSubject -and $vcJson.credentialSubject.id) {
      $vcJson.credentialSubject.id = ($vcJson.credentialSubject.id + '-tampered')
    } else {
      Write-Warn "VC structure unknown; appending field to force mismatch"
      $vcJson | Add-Member -NotePropertyName tampered -NotePropertyValue $true -Force
    }
    $vcJson | ConvertTo-Json -Depth 32 | Set-Content -LiteralPath $credFile -Encoding UTF8
  }

  # Commit (auto-register) via wallet-cli (node_url must include /api)
  Write-Step "Committing VC via wallet-cli (auto-register)"
  # Run without piping to preserve $LASTEXITCODE
  & $walletExe vc-commit --file $credFile --dir $credDir --node-url "$base/api" --issuer-endpoint $base
  $commitExit = $LASTEXITCODE

  if ($NegativeMode -eq 'None') {
    if ($commitExit -ne 0) { throw "vc-commit failed unexpectedly with exit code $commitExit" }

    # Confirm GET
    Write-Step "Confirming GET /api/identity/commitments/$hash"
    $get = Invoke-RestMethod -Uri "$base/api/identity/commitments/$hash" -Method GET
    if (-not $get.issuer_did) { throw "Missing issuer_did in GET response" }
    Write-Ok "Commitment present. issuer_did=$($get.issuer_did) did=$($get.did)"
    Write-Host "\nE2E identity flow completed successfully." -ForegroundColor Green
  }
  elseif ($NegativeMode -eq 'DisallowedIssuer') {
    if ($commitExit -eq 0) { throw "Expected vc-commit to fail for DisallowedIssuer, but it exited 0" }
    Write-Ok "vc-commit failed as expected (DisallowedIssuer). Exit=$commitExit"
    Write-Step "Checking that commitment is not publicly available (expect 404)"
    try {
      $null = Invoke-WebRequest -Uri "$base/api/identity/commitments/$hash" -Method GET -UseBasicParsing
      throw "GET returned success but should be 404"
    } catch {
      if ($_.Exception.Response.StatusCode.value__ -ne 404) {
        throw "Expected 404, got $($_.Exception.Response.StatusCode)"
      }
    }
    Write-Host "\nNegative E2E (DisallowedIssuer) completed successfully." -ForegroundColor Green
  }
  elseif ($NegativeMode -eq 'InvalidSignature') {
    if ($commitExit -eq 0) { throw "Expected vc-commit to fail for InvalidSignature, but it exited 0" }
    Write-Ok "vc-commit failed as expected (InvalidSignature). Exit=$commitExit"
    Write-Step "Checking that commitment is not publicly available (expect 404)"
    try {
      $null = Invoke-WebRequest -Uri "$base/api/identity/commitments/$hash" -Method GET -UseBasicParsing
      throw "GET returned success but should be 404"
    } catch {
      if ($_.Exception.Response.StatusCode.value__ -ne 404) {
        throw "Expected 404, got $($_.Exception.Response.StatusCode)"
      }
    }
    Write-Host "\nNegative E2E (InvalidSignature) completed successfully." -ForegroundColor Green
  }
}
finally {
  if ($server -and (Get-Process -Id $server.Id -ErrorAction SilentlyContinue)) {
    Write-Step "Stopping blockchain-server (pid $($server.Id))"
    Stop-Process -Id $server.Id -Force
  }
}
