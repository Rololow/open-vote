@echo off
setlocal enabledelayedexpansion

set BUILD=false
for %%A in (%*) do (
  if /I "%%~A"=="-Build" set BUILD=true
)

if /I "%BUILD%"=="true" (
  echo ==^> Building in release mode
  cargo build --release -p blockchain-server
  if ERRORLEVEL 1 (
    echo Build failed
    exit /b 1
  )
)

set BIN=target\release\blockchain-server.exe
if not exist "%BIN%" (
  echo Binary not found: %BIN%
  echo Run with -Build to compile first.
  exit /b 1
)

set configs=node1-config.json node2-config.json node3-config.json
set i=0
for %%C in (%configs%) do (
  set /a i+=1
  set CFG=%%C
  if exist "!CFG!" (
    echo Starting node !i! with !CFG!
    start "node-!i!" "%CD%\%BIN%" !CFG!
  ) else (
    echo Missing config !CFG!
  )
)

echo All nodes launched. Press any key to finish (they keep running)...
pause >nul
