#!/bin/bash

# 🧪 Script d'Exécution des Tests Système Persistant
# Utilise Docker pour éviter les problèmes de compilation Windows

set -e

echo "🚀 Tests Système d'Utilisateurs Persistants"
echo "=========================================="

# Couleurs pour output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Fonction utilitaire pour logs colorés
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Vérifier que Docker est disponible
if ! command -v docker &> /dev/null; then
    log_error "Docker n'est pas installé ou disponible"
    exit 1
fi

log_info "Docker détecté - Version: $(docker --version)"

# Configuration
DOCKER_IMAGE="rust:1.70"
WORKSPACE_PATH="/workspace"
PROJECT_PATH="$(pwd)"

# Fonction pour exécuter tests dans Docker
run_docker_test() {
    local test_name=$1
    local test_pattern=$2
    
    log_info "Exécution: $test_name"
    
    docker run --rm \
        -v "$PROJECT_PATH:$WORKSPACE_PATH" \
        -w "$WORKSPACE_PATH" \
        "$DOCKER_IMAGE" \
        bash -c "cargo test --manifest-path Cargo.toml $test_pattern -- --nocapture --test-threads=2"
    
    if [ $? -eq 0 ]; then
        log_success "$test_name - RÉUSSI"
    else
        log_error "$test_name - ÉCHEC"
        return 1
    fi
}

# Fonction pour build et check compilation
check_compilation() {
    log_info "Vérification compilation (cargo check)"
    
    docker run --rm \
        -v "$PROJECT_PATH:$WORKSPACE_PATH" \
        -w "$WORKSPACE_PATH" \
        "$DOCKER_IMAGE" \
        cargo check --workspace
    
    if [ $? -eq 0 ]; then
        log_success "Compilation - OK"
    else
        log_error "Compilation - Échec"
        return 1
    fi
}

# Menu principal
show_menu() {
    echo ""
    echo "📋 Options disponibles:"
    echo "1) Vérifier compilation"
    echo "2) Tests d'intégration complets"
    echo "3) Tests de charge et stress"
    echo "4) Tests cryptographiques"
    echo "5) Tests de performance"
    echo "6) Tous les tests persistants"
    echo "7) Tests individuels (menu détaillé)"
    echo "8) Rapport de couverture"
    echo "9) Quitter"
    echo ""
}

# Tests individuels détaillés
run_individual_tests() {
    echo ""
    echo "🔍 Tests individuels disponibles:"
    echo "1) test_complete_user_workflow"
    echo "2) test_identity_validation_security"
    echo "3) test_concurrent_user_registration"
    echo "4) test_cryptographic_key_generation_security"
    echo "5) test_authentication_performance"
    echo "6) Retour au menu principal"
    
    read -p "Choisissez un test (1-6): " test_choice
    
    case $test_choice in
        1)
            run_docker_test "Workflow Utilisateur Complet" "--test persistent_integration_tests test_complete_user_workflow"
            ;;
        2)
            run_docker_test "Sécurité Validation Identité" "--test persistent_integration_tests test_identity_validation_security"
            ;;
        3)
            run_docker_test "Inscriptions Concurrentes" "--test persistent_load_tests test_concurrent_user_registration"
            ;;
        4)
            run_docker_test "Sécurité Génération Clés" "--test persistent_crypto_tests test_cryptographic_key_generation_security"
            ;;
        5)
            run_docker_test "Performance Authentification" "--test persistent_performance_tests test_authentication_performance"
            ;;
        6)
            return
            ;;
        *)
            log_warning "Option invalide"
            ;;
    esac
}

# Fonction pour générer rapport de couverture
generate_coverage_report() {
    log_info "Génération rapport de couverture"
    
    docker run --rm \
        -v "$PROJECT_PATH:$WORKSPACE_PATH" \
        -w "$WORKSPACE_PATH" \
        "$DOCKER_IMAGE" \
        bash -c "
            cargo install cargo-tarpaulin && \
            cargo tarpaulin --out Html --output-dir coverage-report --skip-clean
        "
    
    if [ $? -eq 0 ]; then
        log_success "Rapport généré dans: coverage-report/tarpaulin-report.html"
    else
        log_warning "Échec génération rapport (optionnel)"
    fi
}

# Boucle menu principal
while true; do
    show_menu
    read -p "Votre choix (1-9): " choice
    
    case $choice in
        1)
            check_compilation
            ;;
        2)
            run_docker_test "Tests d'Intégration" "--test persistent_integration_tests"
            ;;
        3)
            run_docker_test "Tests de Charge" "--test persistent_load_tests"
            ;;
        4)
            run_docker_test "Tests Cryptographiques" "--test persistent_crypto_tests"
            ;;
        5)
            run_docker_test "Tests de Performance" "--test persistent_performance_tests"
            ;;
        6)
            log_info "Exécution de tous les tests persistants..."
            run_docker_test "Tous les Tests d'Intégration" "--test persistent_integration_tests" &&
            run_docker_test "Tous les Tests de Charge" "--test persistent_load_tests" &&
            run_docker_test "Tous les Tests Cryptographiques" "--test persistent_crypto_tests" &&
            run_docker_test "Tous les Tests de Performance" "--test persistent_performance_tests"
            
            if [ $? -eq 0 ]; then
                log_success "🎉 TOUS LES TESTS RÉUSSIS!"
                echo ""
                echo "📊 Résumé:"
                echo "✅ Tests d'intégration: OK"
                echo "✅ Tests de charge: OK"
                echo "✅ Tests cryptographiques: OK"
                echo "✅ Tests de performance: OK"
                echo ""
                echo "🚀 Le système est prêt pour la production!"
            fi
            ;;
        7)
            run_individual_tests
            ;;
        8)
            generate_coverage_report
            ;;
        9)
            log_info "Au revoir!"
            exit 0
            ;;
        *)
            log_warning "Option invalide. Veuillez choisir entre 1 et 9."
            ;;
    esac
    
    echo ""
    read -p "Appuyez sur Entrée pour continuer..."
done