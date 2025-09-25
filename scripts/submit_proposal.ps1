<#!
.SYNOPSIS
  Soumettre rapidement une proposition de loi (proposition citoyenne) au serveur blockchain.

.DESCRIPTION
  Script PowerShell utilitaire pour envoyer une requête POST JSON vers l'endpoint /api/proposals
  Les valeurs par défaut visent http://127.0.0.1:3000. Personnalisez avec les paramètres.

.PARAMETER Title
  Titre de la proposition.

.PARAMETER Category
  Catégorie (ex: Justice, Education, Sante, Environnement)

.PARAMETER Description
  Résumé court de la proposition.

.PARAMETER FullText
  Texte complet (optionnel)

.PARAMETER Host
  Host (par défaut 127.0.0.1)

.PARAMETER Port
  Port API (par défaut 3000)

.PARAMETER AuthorId
  Identifiant auteur optionnel

.PARAMETER AuthorName
  Nom de l'auteur optionnel

.PARAMETER Tags
  Liste de tags séparés par des virgules

.EXAMPLE
  ./submit_proposal.ps1 -Title "Protection des forêts" -Category Environnement -Description "Renforcer les sanctions..." -Tags "foret,ecologie,climat"

#>
[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)] [string]$Title,
  [Parameter(Mandatory=$true)] [string]$Category,
  [Parameter(Mandatory=$true)] [string]$Description,
  [string]$FullText = "",
  [string]$Host = "127.0.0.1",
  [int]$Port = 3000,
  [string]$AuthorId,
  [string]$AuthorName,
  [string]$Tags
)

$ErrorActionPreference = 'Stop'

$body = [ordered]@{
  title = $Title
  category = $Category
  description = $Description
  full_text = $FullText
}
if ($AuthorId) { $body.author_id = $AuthorId }
if ($AuthorName) { $body.author_name = $AuthorName }
if ($Tags) { $body.tags = $Tags.Split(',') | ForEach-Object { $_.Trim() } }

$uri = "http://$Host:$Port/api/proposals"
Write-Host "POST $uri" -ForegroundColor Cyan
Write-Host (ConvertTo-Json $body -Depth 4) -ForegroundColor DarkGray

try {
  $response = Invoke-RestMethod -Uri $uri -Method Post -Body ($body | ConvertTo-Json -Depth 4 -Encoding UTF8) -ContentType 'application/json; charset=utf-8'
  Write-Host "Réponse:" -ForegroundColor Green
  $response | ConvertTo-Json -Depth 6
} catch {
  Write-Host "Erreur appel API: $($_.Exception.Message)" -ForegroundColor Red
  if ($_.ErrorDetails.Message) { Write-Host $_.ErrorDetails.Message -ForegroundColor Red }
  exit 1
}
