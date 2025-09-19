# 🐳 Guide de Déploiement Docker

## Pourquoi Docker ?

Le projet E-Government Blockchain rencontre des difficultés de compilation sur Windows dues aux problèmes de linker (MSVC/GNU). Docker offre une solution élégante en utilisant un environnement Linux standardisé.

## 📋 Prérequis

```bash
# Installer Docker Desktop pour Windows
winget install Docker.DockerDesktop

# Redémarrer le système après installation
```

## 🏗️ Configuration Docker

### 1. Dockerfile Principal

```dockerfile
# Dockerfile
FROM rust:1.75-slim-bookworm

# Installer les dépendances système
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    sqlite3 \
    libsqlite3-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Créer un utilisateur non-root
RUN useradd -m -u 1000 rustuser
USER rustuser

# Définir le répertoire de travail
WORKDIR /app

# Copier les fichiers de configuration Cargo
COPY --chown=rustuser:rustuser Cargo.toml Cargo.lock ./
COPY --chown=rustuser:rustuser crypto-lib/Cargo.toml ./crypto-lib/
COPY --chown=rustuser:rustuser common/Cargo.toml ./common/
COPY --chown=rustuser:rustuser blockchain-server/Cargo.toml ./blockchain-server/
COPY --chown=rustuser:rustuser api-gateway/Cargo.toml ./api-gateway/
COPY --chown=rustuser:rustuser web-interface/Cargo.toml ./web-interface/
COPY --chown=rustuser:rustuser tests/Cargo.toml ./tests/

# Créer les répertoires src avec des fichiers dummy pour la pré-compilation
RUN mkdir -p crypto-lib/src common/src blockchain-server/src api-gateway/src web-interface/src tests/src && \
    echo "fn main() {}" > crypto-lib/src/lib.rs && \
    echo "fn main() {}" > common/src/lib.rs && \
    echo "fn main() {}" > blockchain-server/src/main.rs && \
    echo "fn main() {}" > api-gateway/src/main.rs && \
    echo "fn main() {}" > web-interface/src/main.rs && \
    echo "fn main() {}" > tests/src/lib.rs

# Pré-compiler les dépendances (optimisation)
RUN cargo build --release
RUN rm -rf crypto-lib/src common/src blockchain-server/src api-gateway/src web-interface/src tests/src

# Copier le code source réel
COPY --chown=rustuser:rustuser . .

# Compiler le projet final
RUN cargo build --release

# Exposer les ports
EXPOSE 8080 8081

# Script de démarrage
CMD ["./target/release/blockchain-server"]
```

### 2. Docker Compose pour l'Orchestration

```yaml
# docker-compose.yml
version: '3.8'

services:
  blockchain-server:
    build: .
    container_name: egovern-blockchain
    ports:
      - "8080:8080"
    volumes:
      - blockchain_data:/app/data
      - blockchain_logs:/app/logs
    environment:
      - RUST_LOG=info
      - DATABASE_URL=./data/blockchain.db
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  api-gateway:
    build: .
    container_name: egovern-gateway
    command: ["./target/release/api-gateway"]
    ports:
      - "8081:8081"
    depends_on:
      - blockchain-server
    environment:
      - RUST_LOG=info
      - BLOCKCHAIN_URL=http://blockchain-server:8080
    restart: unless-stopped

  web-interface:
    build: .
    container_name: egovern-web
    command: ["./target/release/web-interface"]
    ports:
      - "3000:3000"
    depends_on:
      - api-gateway
    environment:
      - API_URL=http://api-gateway:8081
    restart: unless-stopped

volumes:
  blockchain_data:
  blockchain_logs:
```

### 3. Script de Démarrage Simplifié

```powershell
# start-docker.ps1
Write-Host "🐳 Démarrage E-Government Blockchain via Docker..." -ForegroundColor Green

# Vérifier que Docker est démarré
if (-not (Get-Process "Docker Desktop" -ErrorAction SilentlyContinue)) {
    Write-Host "⚠️ Docker Desktop n'est pas démarré. Veuillez le lancer manuellement." -ForegroundColor Yellow
    Start-Process "Docker Desktop"
    Read-Host "Appuyez sur Entrée quand Docker Desktop est prêt..."
}

# Construire et démarrer les services
Write-Host "🏗️ Construction des images Docker..." -ForegroundColor Blue
docker-compose build

Write-Host "🚀 Démarrage des services..." -ForegroundColor Blue
docker-compose up -d

Write-Host "📊 Status des services..." -ForegroundColor Blue
docker-compose ps

Write-Host "✅ Services démarrés !" -ForegroundColor Green
Write-Host "🌐 Blockchain Server: http://localhost:8080" -ForegroundColor Cyan
Write-Host "🌐 API Gateway: http://localhost:8081" -ForegroundColor Cyan
Write-Host "🌐 Web Interface: http://localhost:3000" -ForegroundColor Cyan

Write-Host "📝 Pour voir les logs: docker-compose logs -f" -ForegroundColor Gray
Write-Host "🛑 Pour arrêter: docker-compose down" -ForegroundColor Gray
```

## 🚀 Utilisation

### Démarrage Rapide

```powershell
# 1. Créer les fichiers Docker (Dockerfile et docker-compose.yml)
# 2. Exécuter le script de démarrage
.\start-docker.ps1

# Ou manuellement :
docker-compose up --build -d
```

### Tests et Vérification

```bash
# Vérifier que les services fonctionnent
curl http://localhost:8080/health
curl http://localhost:8081/api/accounts

# Voir les logs
docker-compose logs blockchain-server
docker-compose logs api-gateway

# Accéder au container pour débugger
docker exec -it egovern-blockchain bash
```

### Arrêt et Nettoyage

```powershell
# Arrêter les services
docker-compose down

# Nettoyer complètement (attention : supprime les données)
docker-compose down -v --rmi all

# Nettoyer uniquement les containers
docker-compose down --remove-orphans
```

## 🔧 Avantages de cette Approche

1. **✅ Compilation garantie** : Environnement Linux standard
2. **🚀 Déploiement facile** : Un seul script PowerShell
3. **🔧 Isolation** : Pas de pollution de l'environnement Windows
4. **📦 Portabilité** : Fonctionne sur Linux, macOS, Windows
5. **🏗️ Production-ready** : Orchestration avec docker-compose
6. **📊 Monitoring** : Health checks et logs centralisés

## 🎯 Prochaines Étapes

1. Créer le `Dockerfile` et `docker-compose.yml`
2. Tester la compilation dans le container Linux
3. Déployer et tester le système complet
4. Ajouter monitoring et métriques (Prometheus/Grafana)
5. Déployer sur un serveur cloud (AWS/Azure/GCP)

Cette approche Docker résout définitivement les problèmes de compilation Windows tout en offrant un environnement de déploiement professionnel ! 🎉
