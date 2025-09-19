# Interface Web E-Government Blockchain

Interface utilisateur moderne pour le système de gouvernance décentralisée E-Government Blockchain, développée avec le framework Yew (Rust + WebAssembly).

## 🚀 Fonctionnalités

### ✅ Implémentées
- **Authentification complète** : Inscription, connexion avec gestion de session
- **Page d'accueil** : Présentation du système et statistiques
- **Consultation des lois** : Recherche, filtrage et visualisation détaillée
- **Système de propositions** : Soumission de nouvelles lois et amendements
- **Interface de vote** : Participation démocratique avec résultats en temps réel
- **Profil utilisateur** : Gestion du compte et statistiques personnelles
- **Design responsive** : Compatible mobile, tablette et desktop
- **Système de style complet** : CSS moderne avec thème cohérent

### 🔄 En développement
- Intégration avec l'API blockchain
- Notifications en temps réel
- Mode sombre
- Progressive Web App (PWA)

## 📁 Structure du projet

```
web-interface/
├── src/
│   ├── app.rs              # Application principale et routage
│   ├── main.rs             # Point d'entrée
│   ├── pages/              # Pages de l'application
│   │   ├── mod.rs         # Module des pages
│   │   ├── auth.rs        # Login et inscription
│   │   ├── home.rs        # Page d'accueil
│   │   ├── laws.rs        # Consultation des lois
│   │   ├── proposals.rs   # Système de propositions
│   │   ├── voting.rs      # Interface de vote
│   │   └── profile.rs     # Profil utilisateur
│   └── styles/            # Styles CSS
│       ├── index.css      # Import principal
│       ├── main.css       # Styles de base
│       ├── auth.css       # Styles authentification
│       ├── home.css       # Styles page d'accueil
│       ├── laws.css       # Styles consultation lois
│       ├── proposals.css  # Styles propositions
│       ├── voting.css     # Styles interface vote
│       └── profile.css    # Styles profil
├── index.html             # Template HTML principal
├── Cargo.toml            # Configuration Rust
└── README.md             # Cette documentation
```

## 🎨 Design System

### Palette de couleurs
- **Primaire** : #2563eb (Bleu)
- **Succès** : #059669 (Vert)
- **Erreur** : #dc2626 (Rouge)
- **Warning** : #d97706 (Orange)
- **Info** : #0891b2 (Cyan)
- **Gris** : Échelle de #f8fafc à #0f172a

### Composants
- **Boutons** : Primary, Secondary, Success, Danger avec états hover/disabled
- **Formulaires** : Validation, états d'erreur, loading
- **Cartes** : Conteneurs avec ombre et bordures arrondies
- **Modal** : Fenêtres modales pour les détails
- **Navigation** : Barre de navigation responsive avec menu mobile

### Responsive Design
- **Mobile** : < 480px
- **Tablette** : 481px - 768px
- **Desktop** : > 768px

## 🔧 Installation et développement

### Prérequis
- Rust (dernière version stable)
- Trunk (pour le développement Yew)
- Node.js (optionnel, pour les outils de développement)

### Installation
```bash
# Installer Trunk
cargo install trunk

# Installer les dépendances
cd web-interface
```

### Développement
```bash
# Lancer le serveur de développement
trunk serve

# Build pour la production
trunk build --release
```

### Serveur de développement
L'application sera disponible sur `http://localhost:8080`

## 📖 Guide d'utilisation

### Navigation
- **Accueil** : Vue d'ensemble du système et statistiques
- **Lois** : Consultation et recherche dans les lois existantes
- **Propositions** : Soumission de nouvelles lois ou amendements
- **Votes** : Participation aux votes démocratiques
- **Profil** : Gestion du compte utilisateur (authentification requise)

### Authentification
1. **Inscription** : Créer un nouveau compte avec nom d'utilisateur, email et mot de passe
2. **Connexion** : Se connecter avec les identifiants
3. **Session** : Maintenue via localStorage

### Fonctionnalités principales

#### Consultation des lois
- Recherche textuelle dans le titre et contenu
- Filtrage par catégorie (Civile, Pénale, Commerciale, etc.)
- Filtrage par statut (Active, Inactive, Brouillon)
- Visualisation détaillée avec modal

#### Système de propositions
- Choix entre nouvelle loi ou amendement
- Formulaire guidé avec validation
- Prévisualisation avant soumission
- Aide contextuelle

#### Interface de vote
- Vue d'ensemble des propositions en cours
- Barres de progression des résultats
- Vote Pour/Contre/Abstention
- Détails des propositions avec historique

#### Profil utilisateur
- Modification des informations personnelles
- Système de réputation
- Historique d'activité
- Statistiques personnelles

## 🔗 Intégration API

### Points d'intégration prévus
- `POST /api/auth/login` - Authentification
- `POST /api/auth/register` - Inscription
- `GET /api/laws` - Liste des lois
- `GET /api/laws/{id}` - Détail d'une loi
- `POST /api/proposals` - Soumission de proposition
- `GET /api/voting/proposals` - Propositions en vote
- `POST /api/voting/{id}/vote` - Soumettre un vote
- `GET /api/user/profile` - Profil utilisateur

### Format des données
Toutes les API utilisent JSON pour l'échange de données, avec gestion d'erreur standardisée.

## 🎯 Roadmap

### Version 1.0
- [x] Interface utilisateur complète
- [x] Système d'authentification
- [x] Toutes les pages fonctionnelles
- [ ] Intégration API blockchain
- [ ] Tests end-to-end

### Version 1.1
- [ ] Notifications en temps réel
- [ ] Mode sombre
- [ ] PWA avec cache offline
- [ ] Optimisations performances

### Version 2.0
- [ ] Système de commentaires
- [ ] Chat communautaire
- [ ] Analyses avancées
- [ ] Import/export de données

## 🤝 Contribution

1. Fork du projet
2. Créer une branche feature (`git checkout -b feature/nouvelle-fonctionnalite`)
3. Commit des changements (`git commit -am 'Ajout nouvelle fonctionnalité'`)
4. Push vers la branche (`git push origin feature/nouvelle-fonctionnalite`)
5. Créer une Pull Request

### Standards de code
- Rust standard (rustfmt)
- Documentation des fonctions publiques
- Tests unitaires pour la logique métier
- CSS organisé par composant

## 📝 Licence

Ce projet est sous licence MIT. Voir le fichier LICENSE pour plus de détails.

## 🆘 Support

Pour obtenir de l'aide :
1. Consulter la documentation technique
2. Ouvrir une issue sur GitHub
3. Contacter l'équipe de développement

---

**Note** : Cette interface est conçue pour fonctionner avec le backend blockchain E-Government. Pour un développement complet, assurez-vous que le serveur blockchain fonctionne sur `http://localhost:8080`.