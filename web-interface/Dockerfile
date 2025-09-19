# Dockerfile pour l'interface web E-Government Blockchain
FROM rust:1.75 as builder

# Installer les dépendances système nécessaires
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Installer wasm-pack et trunk
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
RUN cargo install trunk --locked

# Ajouter la target WebAssembly
RUN rustup target add wasm32-unknown-unknown

# Créer le répertoire de travail
WORKDIR /app

# Copier d'abord les dépendances communes
COPY ../common ./common
COPY ../crypto-lib ./crypto-lib
COPY ../Cargo.toml ./workspace-Cargo.toml

# Copier les fichiers de configuration de l'interface web
COPY Cargo.toml ./
COPY index.html ./
COPY Trunk.toml ./

# Copier le code source
COPY src ./src

# Build de l'application
RUN trunk build --release

# Image finale avec serveur web léger
FROM nginx:alpine

# Copier les fichiers buildés
COPY --from=builder /app/dist /usr/share/nginx/html

# Configuration nginx pour SPA
COPY <<EOF /etc/nginx/conf.d/default.conf
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    # Gestion des routes SPA
    location / {
        try_files \$uri \$uri/ /index.html;
    }

    # Cache pour les assets statiques
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|wasm)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # Headers de sécurité
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    
    # CORS pour l'API blockchain
    location /api/ {
        add_header Access-Control-Allow-Origin *;
        add_header Access-Control-Allow-Methods "GET, POST, PUT, DELETE, OPTIONS";
        add_header Access-Control-Allow-Headers "Content-Type, Authorization";
        
        if (\$request_method = OPTIONS) {
            return 204;
        }
        
        # Proxy vers le serveur blockchain
        proxy_pass http://blockchain-server:8080;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]