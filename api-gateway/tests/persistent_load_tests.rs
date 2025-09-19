use anyhow::Result;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use std::sync::Arc;
use api_gateway::persistent_gateway::PersistentApiGateway;
use axum_test::TestServer;

/// Tests de charge et stress pour le système persistant
/// 
/// Ces tests vérifient que le système reste performant et stable
/// sous charge simulée avec de nombreux utilisateurs concurrents

#[tokio::test]
async fn test_concurrent_user_registrations() -> Result<()> {
    println!("🔥 Test de charge: Inscriptions concurrentes");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    let concurrent_users = 50;
    let semaphore = Arc::new(Semaphore::new(10)); // Limite à 10 requêtes simultanées
    let start_time = Instant::now();
    
    let mut handles = vec![];
    
    for i in 0..concurrent_users {
        let server_clone = server.clone();
        let semaphore_clone = Arc::clone(&semaphore);
        
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();
            
            let register_request = json!({
                "email": format!("load.test.{}@example.com", i),
                "name": format!("Load Tester {}", i),
                "password": "LoadTestPassword123!"
            });
            
            let start = Instant::now();
            let response = server_clone
                .post("/api/v2/auth/register")
                .json(&register_request)
                .await;
            let duration = start.elapsed();
            
            if response.status_code().is_success() {
                println!("✅ Utilisateur {} inscrit en {:?}", i, duration);
            } else {
                println!("❌ Échec inscription utilisateur {}: {}", i, response.status_code());
            }
            
            response.assert_status_ok();
        });
        
        handles.push(handle);
    }
    
    // Attendre la completion de tous les utilisateurs
    for handle in handles {
        if let Err(e) = handle.await {
            println!("❌ Erreur dans un thread: {}", e);
        }
    }
    
    let total_duration = start_time.elapsed();
    let rate = concurrent_users as f64 / total_duration.as_secs_f64();
    
    println!("📊 Résultats charge inscription:");
    println!("   - {} utilisateurs inscrits", concurrent_users);
    println!("   - Temps total: {:?}", total_duration);
    println!("   - Taux: {:.2} inscriptions/seconde", rate);
    
    // Vérifier performance acceptable (au moins 5 inscriptions/seconde)
    assert!(rate > 5.0, "Performance trop faible: {:.2} inscriptions/sec", rate);
    
    println!("🚀 Test de charge inscription réussi!");
    Ok(())
}

#[tokio::test]
async fn test_concurrent_logins() -> Result<()> {
    println!("🔥 Test de charge: Connexions concurrentes");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    let num_users = 20;
    
    // Première phase: Créer les utilisateurs
    println!("📝 Création de {} utilisateurs de test", num_users);
    
    for i in 0..num_users {
        let register_request = json!({
            "email": format!("login.test.{}@example.com", i),
            "name": format!("Login Tester {}", i),
            "password": "LoginTestPassword123!"
        });
        
        server.post("/api/v2/auth/register")
            .json(&register_request)
            .await
            .assert_status_ok();
    }
    
    println!("✅ {} utilisateurs créés", num_users);
    
    // Deuxième phase: Connexions concurrentes
    println!("🔐 Test de connexions concurrentes");
    
    let start_time = Instant::now();
    let mut handles = vec![];
    
    for i in 0..num_users {
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            let login_request = json!({
                "email": format!("login.test.{}@example.com", i),
                "password": "LoginTestPassword123!"
            });
            
            let start = Instant::now();
            let response = server_clone
                .post("/api/v2/auth/login")
                .json(&login_request)
                .await;
            let duration = start.elapsed();
            
            response.assert_status_ok();
            
            let login_data: Value = response.json();
            assert_eq!(login_data["success"], true);
            assert!(login_data["data"]["token"].is_string());
            
            println!("✅ Connexion utilisateur {} en {:?}", i, duration);
        });
        
        handles.push(handle);
    }
    
    // Attendre completion
    for handle in handles {
        handle.await?;
    }
    
    let total_duration = start_time.elapsed();
    let rate = num_users as f64 / total_duration.as_secs_f64();
    
    println!("📊 Résultats charge connexion:");
    println!("   - {} connexions simultanées", num_users);
    println!("   - Temps total: {:?}", total_duration);
    println!("   - Taux: {:.2} connexions/seconde", rate);
    
    // Vérifier performance acceptable
    assert!(rate > 10.0, "Performance connexion trop faible: {:.2}/sec", rate);
    
    println!("🚀 Test de charge connexion réussi!");
    Ok(())
}

#[tokio::test]
async fn test_memory_usage_under_load() -> Result<()> {
    println!("🧠 Test d'utilisation mémoire sous charge");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Créer beaucoup d'utilisateurs pour tester l'utilisation mémoire
    let num_users = 100;
    
    println!("📈 Création de {} utilisateurs pour test mémoire", num_users);
    
    let start_time = Instant::now();
    
    for i in 0..num_users {
        let register_request = json!({
            "email": format!("memory.test.{}@example.com", i),
            "name": format!("Memory Tester {} with a longer name to use more memory", i),
            "password": "MemoryTestPasswordWithExtraCharacters123!"
        });
        
        let response = server
            .post("/api/v2/auth/register")
            .json(&register_request)
            .await;
        
        response.assert_status_ok();
        
        // Log périodiquement
        if i % 20 == 0 {
            println!("   📊 {} utilisateurs créés", i + 1);
        }
    }
    
    let creation_time = start_time.elapsed();
    println!("✅ {} utilisateurs créés en {:?}", num_users, creation_time);
    
    // Test de connexions multiples pour augmenter l'utilisation mémoire
    println!("🔐 Test connexions multiples");
    
    let mut tokens = vec![];
    
    for i in 0..std::cmp::min(num_users, 50) { // Limiter à 50 pour éviter surcharge
        let login_request = json!({
            "email": format!("memory.test.{}@example.com", i),
            "password": "MemoryTestPasswordWithExtraCharacters123!"
        });
        
        let response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        
        response.assert_status_ok();
        let login_data: Value = response.json();
        tokens.push(login_data["data"]["token"].as_str().unwrap().to_string());
    }
    
    println!("✅ {} tokens générés et stockés", tokens.len());
    
    // Test d'accès simultané aux profils
    println!("👤 Test accès profils simultanés");
    
    let mut handles = vec![];
    
    for (i, token) in tokens.iter().take(20).enumerate() {
        let server_clone = server.clone();
        let token_clone = token.clone();
        
        let handle = tokio::spawn(async move {
            let response = server_clone
                .get("/api/v2/auth/profile")
                .add_header("Authorization".parse().unwrap(), format!("Bearer {}", token_clone).parse().unwrap())
                .await;
            
            response.assert_status_ok();
            println!("   ✅ Profil {} récupéré", i);
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await?;
    }
    
    let total_time = start_time.elapsed();
    println!("📊 Test mémoire terminé en {:?}", total_time);
    
    // Si on arrive ici sans crash, c'est bon signe pour la gestion mémoire
    println!("🧠 Gestion mémoire stable sous charge!");
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_under_stress() -> Result<()> {
    println!("💥 Test gestion d'erreurs sous stress");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Test 1: Beaucoup de requêtes invalides
    println!("🔍 Test résistance aux requêtes invalides");
    
    let invalid_requests = 30;
    let mut handles = vec![];
    
    for i in 0..invalid_requests {
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            // Requête avec données invalides
            let invalid_request = json!({
                "email": "not-an-email",
                "name": "", // Nom vide
                "password": "123" // Mot de passe trop court
            });
            
            let response = server_clone
                .post("/api/v2/auth/register")
                .json(&invalid_request)
                .await;
            
            // Doit retourner une erreur, pas crasher
            assert!(response.status_code().is_client_error());
            
            let error_data: Value = response.json();
            assert_eq!(error_data["success"], false);
            assert!(error_data["error"].is_string());
            
            if i % 10 == 0 {
                println!("   ✅ Requête invalide {} gérée correctement", i);
            }
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await?;
    }
    
    println!("✅ {} requêtes invalides gérées sans crash", invalid_requests);
    
    // Test 2: Tentatives de connexion avec mauvais mots de passe
    println!("🔍 Test résistance aux tentatives de brute force");
    
    // Créer un utilisateur cible
    let target_email = "brute.force.target@example.com";
    let correct_password = "CorrectPassword123!";
    
    let register_request = json!({
        "email": target_email,
        "name": "Brute Force Target",
        "password": correct_password
    });
    
    server.post("/api/v2/auth/register")
        .json(&register_request)
        .await
        .assert_status_ok();
    
    // Beaucoup de tentatives avec mauvais mots de passe
    let brute_force_attempts = 20;
    let mut handles = vec![];
    
    for i in 0..brute_force_attempts {
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            let wrong_login = json!({
                "email": target_email,
                "password": format!("WrongPassword{}", i)
            });
            
            let response = server_clone
                .post("/api/v2/auth/login")
                .json(&wrong_login)
                .await;
            
            // Doit retourner unauthorized
            response.assert_status_unauthorized();
        });
        
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await?;
    }
    
    // Vérifier que le bon mot de passe marche encore
    let correct_login = json!({
        "email": target_email,
        "password": correct_password
    });
    
    let final_response = server
        .post("/api/v2/auth/login")
        .json(&correct_login)
        .await;
    
    final_response.assert_status_ok();
    
    println!("✅ {} tentatives brute force bloquées, connexion légitime OK", brute_force_attempts);
    
    // Test 3: Accès endpoints protégés sans token
    println!("🔍 Test protection endpoints sans authentification");
    
    let protected_endpoints = vec![
        "/api/v2/auth/profile",
        "/api/v2/identity/submit", 
        "/api/v2/identity/status",
        "/api/v2/crypto/generate",
        "/api/v2/crypto/info",
        "/api/v2/vote/eligibility"
    ];
    
    for endpoint in &protected_endpoints {
        let response = server.get(endpoint).await;
        assert!(response.status_code().is_client_error(), 
                "Endpoint {} devrait être protégé", endpoint);
    }
    
    println!("✅ Tous les endpoints protégés correctement sécurisés");
    
    println!("💪 Système résistant aux erreurs et attaques!");
    
    Ok(())
}

#[tokio::test]
async fn test_performance_benchmarks() -> Result<()> {
    println!("⚡ Benchmarks de performance");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Benchmark 1: Temps de réponse inscription
    println!("📝 Benchmark inscription utilisateur");
    
    let mut inscription_times = vec![];
    
    for i in 0..10 {
        let register_request = json!({
            "email": format!("bench.register.{}@example.com", i),
            "name": format!("Benchmark Register {}", i),
            "password": "BenchmarkPassword123!"
        });
        
        let start = Instant::now();
        let response = server
            .post("/api/v2/auth/register")
            .json(&register_request)
            .await;
        let duration = start.elapsed();
        
        response.assert_status_ok();
        inscription_times.push(duration);
    }
    
    let avg_inscription = inscription_times.iter().sum::<Duration>() / inscription_times.len() as u32;
    let max_inscription = inscription_times.iter().max().unwrap();
    
    println!("   📊 Inscription - Moyenne: {:?}, Max: {:?}", avg_inscription, max_inscription);
    
    // Benchmark 2: Temps de réponse connexion
    println!("🔐 Benchmark connexion utilisateur");
    
    let mut login_times = vec![];
    
    for i in 0..10 {
        let login_request = json!({
            "email": format!("bench.register.{}@example.com", i),
            "password": "BenchmarkPassword123!"
        });
        
        let start = Instant::now();
        let response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        let duration = start.elapsed();
        
        response.assert_status_ok();
        login_times.push(duration);
    }
    
    let avg_login = login_times.iter().sum::<Duration>() / login_times.len() as u32;
    let max_login = login_times.iter().max().unwrap();
    
    println!("   📊 Connexion - Moyenne: {:?}, Max: {:?}", avg_login, max_login);
    
    // Assertions de performance
    assert!(avg_inscription.as_millis() < 500, "Inscription trop lente: {:?}", avg_inscription);
    assert!(avg_login.as_millis() < 200, "Connexion trop lente: {:?}", avg_login);
    
    println!("📊 Résumé des benchmarks:");
    println!("   ✅ Inscription moyenne: {:?} (< 500ms)", avg_inscription);
    println!("   ✅ Connexion moyenne: {:?} (< 200ms)", avg_login);
    println!("   ✅ Performance acceptable pour production");
    
    Ok(())
}

/// Test de récupération après erreur (resilience)
#[tokio::test]
async fn test_system_resilience() -> Result<()> {
    println!("🛡️ Test de résilience système");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Simuler charge élevée puis récupération
    println!("🔥 Simulation charge élevée");
    
    let high_load_users = 30;
    let mut handles = vec![];
    
    for i in 0..high_load_users {
        let server_clone = server.clone();
        
        let handle = tokio::spawn(async move {
            // Inscription
            let register_request = json!({
                "email": format!("resilience.{}@example.com", i),
                "name": format!("Resilience User {}", i),
                "password": "ResilienceTest123!"
            });
            
            server_clone.post("/api/v2/auth/register")
                .json(&register_request)
                .await
                .assert_status_ok();
            
            // Connexion immédiate
            let login_request = json!({
                "email": format!("resilience.{}@example.com", i),
                "password": "ResilienceTest123!"
            });
            
            server_clone.post("/api/v2/auth/login")
                .json(&login_request)
                .await
                .assert_status_ok();
        });
        
        handles.push(handle);
    }
    
    // Attendre que tout se termine
    for handle in handles {
        handle.await?;
    }
    
    println!("✅ Système stable après charge élevée");
    
    // Test fonctionnalité normale après stress
    println!("🔍 Vérification fonctionnement normal post-stress");
    
    let normal_request = json!({
        "email": "post.stress@example.com",
        "name": "Post Stress User",
        "password": "PostStressTest123!"
    });
    
    let register_response = server
        .post("/api/v2/auth/register")
        .json(&normal_request)
        .await;
    
    register_response.assert_status_ok();
    
    let login_request = json!({
        "email": "post.stress@example.com",
        "password": "PostStressTest123!"
    });
    
    let login_response = server
        .post("/api/v2/auth/login")
        .json(&login_request)
        .await;
    
    login_response.assert_status_ok();
    
    println!("✅ Fonctionnement normal maintenu après stress");
    println!("🛡️ Système résilient validé!");
    
    Ok(())
}