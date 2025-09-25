# Build all workspace crates in release mode
param(
    [switch]$VerboseLogging
)

Write-Host "==> Building workspace in release mode..." -ForegroundColor Cyan

$env:RUSTFLAGS="-C target-cpu=native"

$cargoCmd = "cargo build --release --workspace"
if ($VerboseLogging){
  $cargoCmd += " -vv"
}

Invoke-Expression $cargoCmd

if ($LASTEXITCODE -ne 0) {
  Write-Host "Build failed" -ForegroundColor Red
  exit 1
}

Write-Host "Build succeeded" -ForegroundColor Green