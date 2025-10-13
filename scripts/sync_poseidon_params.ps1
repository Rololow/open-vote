#!/usr/bin/env pwsh
# Synchronize Poseidon parameters between wallet-cli and blockchain-server
# Usage: pwsh -File sync_poseidon_params.ps1

$walletDataDir = "wallet-cli/zkp"
$serverDataDir = "blockchain-server/zkp"
$poseidonFile = "poseidon_params.bin"

# Source: wallet-cli/zkp/poseidon_params.bin
$sourcePath = Join-Path $walletDataDir $poseidonFile
# Destination: blockchain-server/zkp/poseidon_params.bin
$destPath = Join-Path $serverDataDir $poseidonFile


if (!(Test-Path $sourcePath)) {
    Write-Host "Source Poseidon params file not found: $sourcePath"
    Write-Host "Attempting to generate Poseidon parameters by running zkp_flow_with_clean_log.ps1..."
    pwsh -File scripts/zkp_flow_with_clean_log.ps1
    if (!(Test-Path $sourcePath)) {
        Write-Host "ERROR: Poseidon params file still not found after running zkp_flow_with_clean_log.ps1."
        exit 1
    }
}

Copy-Item -Path $sourcePath -Destination $destPath -Force
Write-Host "Poseidon parameters synchronized: $sourcePath -> $destPath"
