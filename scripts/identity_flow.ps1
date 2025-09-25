param(
    [string]$IssuerEndpoint = "http://localhost:8080",
    [string]$KeyFile = "private_key.pem",
    [string]$OutDir = "$HOME/.e-gov-wallet/credentials"
)

$ErrorActionPreference = "Stop"

function Ensure-Key {
    if (-not (Test-Path $KeyFile)) {
        Write-Host "[+] Génération d'une clé privée (hex) -> $KeyFile"
        cargo run -q -p wallet-cli -- generate-keypair --output $KeyFile | Out-Host
    } else {
        Write-Host "[=] Clé privée trouvée: $KeyFile"
    }
}

function Derive-Did {
    Write-Host "[+] Dérivation du did:key depuis la clé privée"
    $out = (cargo run -q -p wallet-cli -- did-generate --key-file $KeyFile) | Out-String
    $did = ($out -split "`n" | Where-Object { $_ -match "^did:key" } | Select-Object -First 1).Trim()
    if (-not $did) { throw "Impossible de dériver le did:key" }
    Write-Host "[✓] did:key = $did"
    return $did
}

function Request-VC($did) {
    Write-Host "[+] Requête d'un VC auprès de $IssuerEndpoint"
    cargo run -q -p wallet-cli -- vc-request --endpoint $IssuerEndpoint --subject-did $did --out-dir $OutDir | Out-Host
}

function Hash-VC($did) {
    Write-Host "[=] Recalcul du hash d'engagement (contrôle)"
    $latest = Get-ChildItem -Path $OutDir -Filter *.json | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $latest) { throw "Aucun VC trouvé dans $OutDir" }
    cargo run -q -p wallet-cli -- vc-hash --file $latest.FullName | Out-Host
}

# Flow
Ensure-Key
$did = Derive-Did
Request-VC -did $did
Hash-VC -did $did

Write-Host "[✓] Flow identité complété."
