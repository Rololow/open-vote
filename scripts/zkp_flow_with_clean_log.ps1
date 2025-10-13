# Erase errors.log before running the ZKP flow
$logPath = "errors.log"
if (Test-Path $logPath) {
    Clear-Content $logPath
}

# Now run the original ZKP flow script

# Generate Poseidon parameters with wallet-cli and copy to server data dir
$cliParamsDir = Join-Path $PSScriptRoot "wallet-cli"
$poseidonParamsPath = Join-Path $cliParamsDir "zkp\poseidon_params.bin"
Write-Host "[STEP] Generating Poseidon parameters with wallet-cli…" -ForegroundColor Cyan
cargo run --release -p wallet-cli --features zkp_groth16 -- zkp-gen-poseidon-params --data-dir $cliParamsDir | Write-Host
Write-Host "[OK] Poseidon parameters generated at $poseidonParamsPath" -ForegroundColor Green

# Copy to the server's actual data directory for this run
$serverDataDir = Join-Path $env:TEMP ("zkp-flow-" + [guid]::NewGuid().ToString('N'))
$serverParamsDir = Join-Path $serverDataDir "zkp"
if (!(Test-Path $serverParamsDir)) { New-Item -ItemType Directory -Force -Path $serverParamsDir | Out-Null }
Copy-Item -Path $poseidonParamsPath -Destination (Join-Path $serverParamsDir "poseidon_params.bin") -Force
Write-Host "[OK] Poseidon parameters copied to $serverParamsDir" -ForegroundColor Green

# Now run the original ZKP flow script
. "$PSScriptRoot\zkp_flow.ps1"
