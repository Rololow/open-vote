#Requires -Version 7.0
<#
.SYNOPSIS
  Compute and snapshot the daily Merkle root for identity commitments.

.DESCRIPTION
  This script wraps the compute_root CLI (in the blockchain-server crate) and writes
  a dated snapshot file with the Merkle root. It also optionally appends to a CSV
  history file with the date, root, and number of non-empty leaves.

.PARAMETER Input
  Path to the commitments log (hex-encoded 32-byte hashes, one per non-empty line).
  Defaults to <repo>\blockchain-server\data\commitments.log

.PARAMETER OutDir
  Directory to write the daily snapshot and optional history file.
  Defaults to <repo>\data\roots

.PARAMETER Release
  Use cargo --release to run the CLI.

.PARAMETER NoCargo
  Run the prebuilt compute_root.exe directly instead of using cargo run.

.PARAMETER AppendHistory
  Append a line to roots.csv with columns: date,root,leaves

.PARAMETER DryRun
  Show what would be done without writing files.

.EXAMPLE
  pwsh -File scripts/compute_commitments_root.ps1

.EXAMPLE
  pwsh -File scripts/compute_commitments_root.ps1 -Release -AppendHistory
#>

param(
  [string]$Input,
  [string]$OutDir,
  [switch]$Release,
  [switch]$NoCargo,
  [switch]$AppendHistory,
  [switch]$DryRun
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Write-Step($msg) { Write-Host "[STEP] $msg" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "[OK]  $msg" -ForegroundColor Green }
function Write-Err($msg)  { Write-Host "[ERR] $msg" -ForegroundColor Red }

try {
  $scriptDir = Split-Path -Parent $PSCommandPath
  $repoRoot = Split-Path -Parent $scriptDir

  if (-not $Input) { $Input = Join-Path $repoRoot 'blockchain-server\data\commitments.log' }
  if (-not $OutDir) { $OutDir = Join-Path $repoRoot 'data\roots' }

  Write-Step "Input: $Input"
  Write-Step "OutDir: $OutDir"

  if (-not (Test-Path -Path $Input -PathType Leaf)) {
    throw "Input file not found: $Input"
  }

  $date = Get-Date -Format 'yyyy-MM-dd'
  $snapshotName = "root_$date.txt"
  $snapshotPath = Join-Path $OutDir $snapshotName
  $historyCsv = Join-Path $OutDir 'roots.csv'

  # Compute leaves count (non-empty lines)
  $leafCount = (Get-Content -LiteralPath $Input | Where-Object { $_.Trim() -ne '' } | Measure-Object).Count

  if (-not $DryRun) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }

  # Build command
  $rootHex = $null
  if ($NoCargo) {
    $profile = if ($Release) { 'release' } else { 'debug' }
    $exe = Join-Path $repoRoot "target\x86_64-pc-windows-msvc\$profile\compute_root.exe"
    if (-not (Test-Path -Path $exe -PathType Leaf)) {
      throw "compute_root.exe not found at $exe. Build it first or omit -NoCargo to use 'cargo run'."
    }
    $args = @('--', $Input, '--out', $snapshotPath)
    Write-Step "Running: $exe $($args -join ' ')"
    if (-not $DryRun) {
      $output = & $exe @args 2>&1
      if ($LASTEXITCODE -ne 0) { throw "compute_root exited with code $LASTEXITCODE. Output: `n$output" }
      $m = $output | Select-String -Pattern 'merkle_root=([0-9a-f]{64})' | Select-Object -First 1
      if ($m) { $rootHex = $m.Matches[0].Groups[1].Value } else { throw "Failed to parse merkle_root from output." }
    }
  } else {
    $cargoArgs = @('run','-p','blockchain-server')
    if ($Release) { $cargoArgs += '--release' }
    $cargoArgs += @('--bin','compute_root','--', $Input, '--out', $snapshotPath)
    Write-Step "Running: cargo $($cargoArgs -join ' ')"
    if (-not $DryRun) {
      $output = & cargo @cargoArgs 2>&1
      if ($LASTEXITCODE -ne 0) { throw "cargo run failed with code $LASTEXITCODE. Output: `n$output" }
      $m = $output | Select-String -Pattern 'merkle_root=([0-9a-f]{64})' | Select-Object -First 1
      if ($m) { $rootHex = $m.Matches[0].Groups[1].Value } else { throw "Failed to parse merkle_root from output." }
    }
  }

  if ($DryRun) {
    Write-Host "[DRY] Would write snapshot to: $snapshotPath" -ForegroundColor Yellow
    Write-Host "[DRY] Would append history to: $historyCsv (leaves=$leafCount)" -ForegroundColor Yellow
    return
  }

  if (-not (Test-Path -LiteralPath $snapshotPath -PathType Leaf)) {
    throw "Snapshot file was not created: $snapshotPath"
  }

  Write-Ok "Snapshot written: $snapshotPath"
  Write-Ok "Root: $rootHex (leaves=$leafCount)"

  if ($AppendHistory) {
    $header = 'date,root,leaves'
    if (-not (Test-Path -LiteralPath $historyCsv -PathType Leaf)) {
      $header | Out-File -FilePath $historyCsv -Encoding ascii -NoNewline:$false
    }
    "$date,$rootHex,$leafCount" | Out-File -FilePath $historyCsv -Encoding ascii -Append -NoNewline:$false
    Write-Ok "History appended: $historyCsv"
  }
}
catch {
  Write-Err $_
  exit 1
}