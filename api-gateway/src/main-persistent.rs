use anyhow::Result;
use api_gateway::persistent_gateway::run_persistent_api_gateway;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Démarrage du système e-gouvernement avec utilisateurs persistants");
    println!("📋 Fonctionnalités:");
    println!("   ✅ Base de données SQLite persistante");
    println!("   ✅ Validation d'identité avec APIs gouvernementales");
    println!("   ✅ Gestion sécurisée des clés cryptographiques Ed25519");
    println!("   ✅ Workflow complet: Inscription → Validation → Clés → Vote");
    println!("   ✅ Sécurisation Argon2 + AES-GCM + Sessions JWT");
    println!("");

    run_persistent_api_gateway().await
}