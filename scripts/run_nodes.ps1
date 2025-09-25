param(
  [switch]$Build,
  [int]$Start=1,
  [int]$Count=3,
  [switch]$NoColor,
  [switch]$Backtrace,
  [int]$ApiPort,
  [switch]$RunIdempotenceTest,
  [int]$ReadyTimeoutSeconds=30,
  [switch]$AutoExitAfterTests
)

$ErrorActionPreference = 'Stop'

# Assurer l'encodage UTF-8 pour éviter la corruption des caractères accentués
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

function Write-Log($path, $text){
  # S'assure que le dossier existe puis écrit en UTF-8
  $dir = Split-Path $path
  if(-not (Test-Path $dir)){ New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $text | Out-File -FilePath $path -Encoding utf8 -Append
}

function Color($text, $color){
  if($NoColor){ return $text }
  return Write-Host $text -ForegroundColor $color
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspaceRoot = Resolve-Path "$scriptDir/.."
Set-Location $workspaceRoot

# Forcer le niveau de log si non défini
if(-not $env:RUST_LOG){ $env:RUST_LOG = 'info' }
if($Backtrace){ $env:RUST_BACKTRACE = '1' }

if($Build){
  Color "==> Building in release mode" Cyan
  cargo build --release -p blockchain-server
}

$candidatePaths = @(
  (Join-Path $workspaceRoot "target/release/blockchain-server.exe"),
  (Join-Path $workspaceRoot "target/x86_64-pc-windows-msvc/release/blockchain-server.exe"),
  (Join-Path $workspaceRoot "target/debug/blockchain-server.exe"),
  (Join-Path $workspaceRoot "target/x86_64-pc-windows-msvc/debug/blockchain-server.exe")
)

$binPath = $null
foreach($c in $candidatePaths){ if(Test-Path $c){ $binPath = $c; break } }

if(-not $binPath){
  throw "Binary not found in expected locations. Run with -Build to compile (paths checked: $($candidatePaths -join ', '))."
}

Color "Using binary: $binPath" Cyan

# Mapping node index -> config file
$configMap = @{
  1 = "node1-config.json"
  2 = "node2-config.json"
  3 = "node3-config.json"
}

# Start processes
$procs = @()
for($i = $Start; $i -lt ($Start + $Count); $i++){
  if(-not $configMap.ContainsKey($i)){
    Color "Skipping node index $i (no config)" Yellow
    continue
  }
  $cfg = $configMap[$i]
  if(!(Test-Path $cfg)){
    Color "Config file $cfg missing" Red
    continue
  }
  $logFile = Join-Path $workspaceRoot "logs/node${i}.log"
  New-Item -ItemType Directory -Force -Path (Split-Path $logFile) | Out-Null
  Color "Starting node $i using $cfg -> logging to $logFile" Green
  Write-Log $logFile ("[" + (Get-Date -Format o) + "] BOOTSTRAP node $i cfg=$cfg bin=$binPath")
  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $binPath
  # Utiliser .Arguments pour compatibilité (ArgumentList non assignable ici)
  $startInfo.Arguments = '"' + $cfg + '"'
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  if($ApiPort){
    $startInfo.EnvironmentVariables["BLOCKCHAIN_API_PORT"] = [string]$ApiPort
  }
  $p = New-Object System.Diagnostics.Process
  $p.StartInfo = $startInfo
  $null = $p.Start()
  if(-not $p -or $p.HasExited){ Write-Log $logFile ("[" + (Get-Date -Format o) + "] FAILED TO START exitCode=" + $p.ExitCode) }
  # Async logging
  Register-ObjectEvent -InputObject $p -EventName OutputDataReceived -Action { if($EventArgs.Data){ $EventArgs.Data | Out-File -FilePath $Event.MessageData -Encoding utf8 -Append } } -MessageData $logFile | Out-Null
  Register-ObjectEvent -InputObject $p -EventName ErrorDataReceived -Action { if($EventArgs.Data){ $EventArgs.Data | Out-File -FilePath $Event.MessageData -Encoding utf8 -Append } } -MessageData $logFile | Out-Null
  $p.BeginOutputReadLine(); $p.BeginErrorReadLine();
  $procs += $p
}

Color "Launched $($procs.Count) node process(es)." Cyan

# If requested, wait for readiness then run the idempotence test
if($RunIdempotenceTest){
  if(-not $ApiPort){
    Color "--RunIdempotenceTest specified but no -ApiPort provided; defaulting to 3000" Yellow
    $ApiPort = 3000
  }
  $readyUrl = "http://localhost:$ApiPort/issuer/jwk"
  Color "Waiting for issuer endpoint: $readyUrl (timeout ${ReadyTimeoutSeconds}s)" Cyan
  $deadline = (Get-Date).AddSeconds($ReadyTimeoutSeconds)
  $ready = $false
  while((Get-Date) -lt $deadline){
    try {
      $resp = Invoke-WebRequest -Uri $readyUrl -UseBasicParsing -TimeoutSec 3 -Method GET
      if($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 500){
        $ready = $true; break
      }
    } catch {}
    Start-Sleep -Milliseconds 500
  }
  if(-not $ready){
    Color "Issuer endpoint not ready within timeout; skipping test." Red
  } else {
    Color "Issuer ready -> running idempotence test" Green
    $testCmd = "cargo test -p integration-tests test_commitment_insertion_idempotent -- --nocapture"
    Color "Executing: $testCmd" Cyan
    Invoke-Expression $testCmd
    $testExit = $LASTEXITCODE
    if($testExit -eq 0){
      Color "Idempotence test PASSED" Green
    } else {
      Color "Idempotence test FAILED (exit $testExit)" Red
    }
    if($AutoExitAfterTests){
      Color "Auto-exit requested; terminating node processes..." Magenta
      foreach($p in $procs){ if(-not $p.HasExited){ $p.Kill() } }
      exit $testExit
    }
  }
}

Color "Press Ctrl+C to stop all nodes." Cyan

# Wait and cleanup on Ctrl+C
try {
  Start-Sleep -Seconds 1
  foreach($p in $procs){ if($p.HasExited){ Color "(early exit) Node PID $($p.Id) code $($p.ExitCode)" Yellow } }
  while($true){
    Start-Sleep -Seconds 2
    foreach($p in $procs){
      if($p.HasExited){
        Color "Node PID $($p.Id) exited with code $($p.ExitCode)" Yellow
      }
    }
  }
}
catch {
  Color "Stopping nodes..." Magenta
  foreach($p in $procs){ if(-not $p.HasExited){ $p.Kill() } }
}
