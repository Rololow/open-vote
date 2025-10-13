Param(
  [int]$ApiPort = 8086,
  [string]$BindAddress = "127.0.0.1",
  [switch]$Build = $true,
  [switch]$SkipCleanup = $false
)

$ErrorActionPreference = 'Stop'

function Write-Step($msg) { Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Write-Ok($msg) { Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Err($msg) { Write-Host "[ERR] $msg" -ForegroundColor Red }

# Paths
$workspace = Split-Path -Parent $PSCommandPath
$root = Split-Path -Parent $workspace
$tempDir = Join-Path $env:TEMP ("zkp-flow-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null

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
  Write-Step "Building server and wallet-cli with ZKP features (debug)…"
  cargo build -p blockchain-server --features zkp_groth16 -p wallet-cli --features zkp_groth16 | Write-Host
  Write-Ok "Build complete"
}

if (-not $walletExe) { throw "wallet-cli binary not found in target paths" }
if (-not $serverExe) { throw "blockchain-server binary not found in target paths" }

$base = "http://${BindAddress}:${ApiPort}"

# Start server with env vars
Write-Step "Starting blockchain-server on $base"
$env:BLOCKCHAIN_BIND_ADDRESS = $BindAddress
$env:BLOCKCHAIN_API_PORT = $ApiPort
$env:BLOCKCHAIN_DATA_DIRECTORY = $tempDir
$env:ALLOWED_ISSUERS_DIDS = ""  # Allow all issuers for dev

$serverJob = Start-Job -ScriptBlock {
  param($exe, $envVars)
  foreach ($k in $envVars.Keys) { [Environment]::SetEnvironmentVariable($k, $envVars[$k]) }
  & $exe
} -ArgumentList $serverExe, @{
  BLOCKCHAIN_BIND_ADDRESS = $env:BLOCKCHAIN_BIND_ADDRESS
  BLOCKCHAIN_API_PORT = $env:BLOCKCHAIN_API_PORT
  BLOCKCHAIN_DATA_DIRECTORY = $env:BLOCKCHAIN_DATA_DIRECTORY
  ALLOWED_ISSUERS_DIDS = $env:ALLOWED_ISSUERS_DIDS
}

# Wait for server to start
Write-Step "Waiting for server to be ready…"
$maxWait = 30
$waited = 0
while ($waited -lt $maxWait) {
  try {
    $response = Invoke-WebRequest -Uri "$base/health" -Method GET -TimeoutSec 2
    if ($response.StatusCode -eq 200) {
      Write-Ok "Server is ready"
      break
    }
  } catch {
    Start-Sleep -Seconds 1
    $waited++
  }
}
if ($waited -ge $maxWait) {
  Write-Err "Server failed to start within ${maxWait}s"
  Stop-Job $serverJob -ErrorAction SilentlyContinue
  Remove-Job $serverJob -ErrorAction SilentlyContinue
  if (-not $SkipCleanup) { Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue }
  exit 1
}

try {
  # Generate ZKP setup (Poseidon params + Groth16 keys)
  Write-Step "Setting up ZKP parameters and keys (vk_version=1)"
  & $walletExe zkp-setup-keys --data-dir $tempDir --vk-version 1
  if ($LASTEXITCODE -ne 0) { throw "ZKP setup failed" }
  Write-Ok "ZKP setup complete"

  # Generate a test proof using wallet CLI
  Write-Step "Generating ZKP proof with wallet CLI"
  $envelopeFile = Join-Path $tempDir "zkp_proof.json"

  # Use the computed values from setup_placeholder_keys (they satisfy the circuit)
  $rootHex = "21a19a77e89d4b4bd96e8f90acb650f51431b7f99c9fb95444168db784bce477"
  $nullifierHex = "04ca7e6c91687e965728192db4e9a0f0a9f411f257b3cd9d009dc5191cf1aa5d"
  $scope = "support"
  $leafHex = "0000000000000000000000000000000000000000000000000000000000000000"
  $secretHex = "0000000000000000000000000000000000000000000000000000000000000000"
  # 8 zero siblings for depth 8
  $merklePath = @(
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000"
  )
  $directions = "01010101"  # alternating left/right as in setup

  & $walletExe zkp-prove `
    --data-dir $tempDir `
    --vk-version 1 `
    --root-hex $rootHex `
    --scope $scope `
    --nullifier-hex $nullifierHex `
    --leaf-hex $leafHex `
    --secret-hex $secretHex `
    --sib $merklePath[0] `
    --sib $merklePath[1] `
    --sib $merklePath[2] `
    --sib $merklePath[3] `
    --sib $merklePath[4] `
    --sib $merklePath[5] `
    --sib $merklePath[6] `
    --sib $merklePath[7] `
    --directions $directions `
    --out $envelopeFile

  if ($LASTEXITCODE -ne 0) { throw "Proof generation failed" }
  Write-Ok "Proof generated and saved to $envelopeFile"

  # Read the envelope and convert proof to hex for API
  Write-Step "Preparing proof envelope for API submission"
  $envelopeJson = Get-Content $envelopeFile -Raw | ConvertFrom-Json
  $proofBinFile = $envelopeJson.proof_file
  $proofBytes = [System.IO.File]::ReadAllBytes($proofBinFile)
  $proofHex = [BitConverter]::ToString($proofBytes).Replace("-", "").ToLower()

  # Create API payload
  $apiPayload = @{
    vk_version = $envelopeJson.vk_version
    public_inputs = $envelopeJson.public_inputs
    proof = $proofHex
  } | ConvertTo-Json

  Write-Ok "API payload prepared"

  # DEBUG: Calcul du hash SHA256 du scope
  $utf8 = [System.Text.Encoding]::UTF8.GetBytes($scope)
  $sha256 = [System.Security.Cryptography.SHA256]::Create()
  $scopeHash = $sha256.ComputeHash($utf8)
  $scopeHashHex = ($scopeHash | ForEach-Object { $_.ToString("x2") }) -join ""
  Write-Host "Scope hash (SHA256, 32 bytes): $scopeHashHex" -ForegroundColor Yellow

  # DEBUG: Affiche le scope hash issu du proof envelope et compare avec le SHA256 local
  $envelopeScopeHash = $envelopeJson.public_inputs[2]
  Write-Host "Scope hash from proof envelope (Fr hex): $envelopeScopeHash" -ForegroundColor Cyan
  if ($envelopeScopeHash -eq $scopeHashHex) {
      Write-Host "Scope hash from envelope == SHA256(scope) (OK)" -ForegroundColor Green
  } else {
      Write-Host "Scope hash from envelope != SHA256(scope) (NORMAL: Rust fait Fr::from_be_bytes_mod_order)" -ForegroundColor Yellow
  }

  # Debug information
  Write-Host "==== DEBUG: ZKP FLOW CHECKS ====" -ForegroundColor Magenta
  Write-Host "wallet-cli path: $walletExe"
  Write-Host "blockchain-server path: $serverExe"
  Write-Host "Temp data dir: $tempDir"
  Write-Host "API base: $base"
  Write-Host "Config file copied: $rootConfig"
  Write-Host "Root hex: $rootHex"
  Write-Host "Nullifier hex: $nullifierHex"
  Write-Host "Leaf hex: $leafHex"
  Write-Host "Secret hex: $secretHex"
  Write-Host "Scope: $scope"
  Write-Host "Merkle path: $($merklePath -join ',')"
  Write-Host "Directions: $directions"
  Write-Host "VK version: 1"
  Write-Host "Proof envelope file: $envelopeFile"
  Write-Host "Proof bin file: $proofBinFile"
  Write-Host "Proof hex (first 64 chars): $($proofHex.Substring(0, [Math]::Min(64, $proofHex.Length)))"
  Write-Host "API payload: $apiPayload"
  Write-Host "VK file in temp dir: $(Join-Path $tempDir 'zkp\vk-groth16-v1.bin')"
  Write-Host "PK file in temp dir: $(Join-Path $tempDir 'zkp\pk-groth16-v1.bin')"
  if (Test-Path (Join-Path $tempDir 'zkp\vk-groth16-v1.bin')) {
      Write-Host "VK file exists in temp dir." -ForegroundColor Green
  } else {
      Write-Host "VK file missing in temp dir!" -ForegroundColor Red
  }
  if (Test-Path (Join-Path $tempDir 'zkp\pk-groth16-v1.bin')) {
      Write-Host "PK file exists in temp dir." -ForegroundColor Green
  } else {
      Write-Host "PK file missing in temp dir!" -ForegroundColor Red
  }
  if (Test-Path $envelopeFile) {
      Write-Host "Proof envelope file exists." -ForegroundColor Green
  } else {
      Write-Host "Proof envelope file missing!" -ForegroundColor Red
  }
  if (Test-Path $proofBinFile) {
      Write-Host "Proof bin file exists." -ForegroundColor Green
  } else {
      Write-Host "Proof bin file missing!" -ForegroundColor Red
  }
  Write-Host "==== END DEBUG CHECKS ====" -ForegroundColor Magenta

  # Submit proof to server for verification
  Write-Step "Submitting proof to server for verification"
  $verifyUrl = "$base/api/identity/verify_zkp"

  try {
    $response = Invoke-WebRequest -Uri $verifyUrl -Method POST -Body $apiPayload -ContentType "application/json" -TimeoutSec 10
    $result = $response.Content | ConvertFrom-Json

    if ($result.ok -eq $true) {
      Write-Ok "Proof verification successful!"
      Write-Host "Server response: $($result | ConvertTo-Json -Depth 3)" -ForegroundColor Green
    } else {
      Write-Err "Proof verification failed: $($result.message)"
      throw "Verification failed"
    }
  } catch {
    Write-Err "API call failed: $($_.Exception.Message)"
    throw
  }

  Write-Step "ZKP end-to-end flow completed successfully!"
  Write-Host "Files created:" -ForegroundColor Cyan
  Write-Host "  - Proof envelope: $envelopeFile" -ForegroundColor White
  Write-Host "  - Proof binary: $proofBinFile" -ForegroundColor White
  Write-Host "  - Data directory: $tempDir" -ForegroundColor White

} finally {
  # Cleanup
  Write-Step "Stopping server"
  Stop-Job $serverJob -ErrorAction SilentlyContinue
  Remove-Job $serverJob -ErrorAction SilentlyContinue

  if (-not $SkipCleanup) {
    Write-Step "Cleaning up temporary files"
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
  } else {
    Write-Warn "Skipping cleanup - temporary files remain in $tempDir"
  }
}

Write-Ok "ZKP flow automation completed successfully"