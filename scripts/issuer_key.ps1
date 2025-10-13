param(
  [string]$KeyPath = "issuer_ed25519_key.json",
  [string]$ExportJwk = "issuer_public_jwk.json",
  [switch]$Rotate,
  [switch]$NoCargo
)

# Usage:
#  pwsh -NoProfile -File scripts/issuer_key.ps1 -Rotate -KeyPath path\issuer.json -ExportJwk path\issuer.jwk.json

$env:ISSUER_KEY_PATH = $KeyPath

if ($Rotate) {
  if (Test-Path $KeyPath) {
    Write-Host "[INFO] Rotating: deleting existing key $KeyPath" -ForegroundColor Yellow
    Remove-Item -Force $KeyPath
  }
}

if ($NoCargo) {
  & "$PSScriptRoot/../target/debug/issuer_key_tool.exe" --key "$KeyPath" --export-jwk "$ExportJwk"
} else {
  cargo run -p blockchain-server --bin issuer_key_tool -- --key "$KeyPath" --export-jwk "$ExportJwk"
}

if ($LASTEXITCODE -ne 0) {
  Write-Error "issuer_key_tool failed with exit code $LASTEXITCODE"
  exit $LASTEXITCODE
}
