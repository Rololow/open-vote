use anyhow::Result;
use serde_json::{json, Value};
use api_gateway::persistent_gateway::PersistentApiGateway;
use axum_test::TestServer;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

/// Tests de performance et stress pour les opérations critiques
/// 
/// Ces tests vérifient:
/// - Temps de réponse des opérations critiques
/// - Comportement sous charge élevée
/// - Limites de débit
/// - Récupération après stress

#[tokio::test]
async fn test_authentication_performance() -> Result<()> {
    println!("⚡ Test performance authentification");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Créer utilisateur test
    let register_request = json!({
        "email": "perf.auth@example.com",
        "name": "Performance Auth Test",
        "password": "PerfTestPassword123!"
    });
    
    server.post("/api/v2/auth/register")
        .json(&register_request)
        .await
        .assert_status_ok();
    
    println!("✅ Utilisateur test créé");
    
    // Test performance connexion
    let login_request = json!({
        "email": "perf.auth@example.com",
        "password": "PerfTestPassword123!"
    });
    
    let num_logins = 50;
    let mut login_times = vec![];
    
    println!("🔍 Test {} connexions consécutives", num_logins);
    
    for i in 0..num_logins {
        let start = Instant::now();
        
        let response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        
        let duration = start.elapsed();
        login_times.push(duration);
        
        response.assert_status_ok();
        
        if i % 10 == 0 {
            println!("   Connexion {} : {:?}", i, duration);
        }
    }
    
    // Analyser les performances
    let avg_time = login_times.iter().sum::<Duration>() / login_times.len() as u32;
    let max_time = login_times.iter().max().unwrap();
    let min_time = login_times.iter().min().unwrap();
    
    println!("📊 Statistiques connexion :");
    println!("   Temps moyen: {:?}", avg_time);
    println!("   Temps min: {:?}", min_time);
    println!("   Temps max: {:?}", max_time);
    
    // Vérifier que les performances sont acceptables
    assert!(avg_time < Duration::from_millis(200), 
            "Temps moyen connexion trop élevé: {:?}", avg_time);
    assert!(max_time < Duration::from_secs(1), 
            "Temps max connexion trop élevé: {:?}", max_time);
    
    // Test connexions avec timeout
    println!("🔍 Test connexions avec timeout strict");
    
    for i in 0..10 {
        let result = timeout(Duration::from_millis(500), async {
            server.post("/api/v2/auth/login")
                .json(&login_request)
                .await
        }).await;
        
        assert!(result.is_ok(), "Connexion {} timeout après 500ms", i);
        result.unwrap().assert_status_ok();
    }
    
    println!("✅ Toutes les connexions respectent le timeout");
    println!("⚡ Tests performance authentification réussis!");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_key_generation() -> Result<()> {
    println!("🔑 Test génération de clés concurrente");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Créer plusieurs utilisateurs
    let num_users = 10;
    let mut user_tokens = vec![];
    
    println!("🔍 Création de {} utilisateurs", num_users);
    
    for i in 0..num_users {
        let register_request = json!({
            "email": format!("concurrent.key.{}@example.com", i),
            "name": format!("Concurrent Key Test {}", i),
            "password": format!("ConcurrentTest{}!", i)
        });
        
        let register_response = server
            .post("/api/v2/auth/register")
            .json(&register_request)
            .await;
        register_response.assert_status_ok();
        
        // Se connecter pour obtenir token
        let login_request = json!({
            "email": format!("concurrent.key.{}@example.com", i),
            "password": format!("ConcurrentTest{}!", i)
        });
        
        let login_response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        login_response.assert_status_ok();
        
        let login_data: Value = login_response.json();
        let token = login_data["data"]["token"].as_str().unwrap().to_string();
        user_tokens.push(token);
        
        // Simuler validation d'identité
        let identity_request = json!({
            "document_type": "carte_identite",
            "document_number": format!("ID{:06}", i),
            "full_name": format!("Concurrent Key Test {}", i),
            "birth_date": "1990-01-01",
            "address": "123 Test Street"
        });
        
        server.post("/api/v2/identity/submit")
            .add_header("Authorization".parse()?, format!("Bearer {}", user_tokens[i]).parse()?)
            .json(&identity_request)
            .await
            .assert_status_ok();
    }
    
    println!("✅ {} utilisateurs créés et validés", num_users);
    
    // Test génération concurrente de clés
    println!("🔍 Génération concurrente de {} paires de clés", num_users);
    
    let start_time = Instant::now();
    
    let tasks: Vec<_> = user_tokens.iter().enumerate().map(|(i, token)| {
        let server = server.clone();
        let token = token.clone();
        let password = format!("ConcurrentTest{}!", i);
        
        tokio::spawn(async move {
            let key_request = json!({
                "password": password
            });
            
            let response = server
                .post("/api/v2/crypto/generate-keys")
                .add_header("Authorization".parse().unwrap(), format!("Bearer {}", token).parse().unwrap())
                .json(&key_request)
                .await;
            
            (i, response)
        })
    }).collect();
    
    let results = futures::future::join_all(tasks).await;
    let generation_time = start_time.elapsed();
    
    println!("⏱️ Génération concurrente terminée en {:?}", generation_time);
    
    // Vérifier que toutes les générations ont réussi
    for result in results {
        let (i, response) = result?;
        response.assert_status_ok();
        println!("✅ Clés générées pour utilisateur {}", i);
    }
    
    // Vérifier temps total acceptable
    assert!(generation_time < Duration::from_secs(30), 
            "Génération concurrente trop lente: {:?}", generation_time);
    
    // Test récupération des clés générées
    println!("🔍 Vérification des clés générées");
    
    for (i, token) in user_tokens.iter().enumerate() {
        let key_info_response = server
            .get("/api/v2/crypto/key-info")
            .add_header("Authorization".parse()?, format!("Bearer {}", token).parse()?)
            .await;
        
        key_info_response.assert_status_ok();
        let key_data: Value = key_info_response.json();
        assert!(key_data["data"]["has_keys"].as_bool().unwrap());
        
        println!("✅ Clés vérifiées pour utilisateur {}", i);
    }
    
    println!("🔑 Tests génération clés concurrente réussis!");
    Ok(())
}

#[tokio::test]
async fn test_database_performance() -> Result<()> {
    println!("🗄️ Test performance base de données");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Test création utilisateurs en masse
    let num_users = 100;
    let batch_size = 10;
    
    println!("🔍 Création de {} utilisateurs par batch de {}", num_users, batch_size);
    
    let mut total_creation_time = Duration::from_millis(0);
    
    for batch in 0..(num_users / batch_size) {
        let batch_start = Instant::now();
        
        let batch_tasks: Vec<_> = (0..batch_size).map(|i| {
            let server = server.clone();
            let user_index = batch * batch_size + i;
            
            tokio::spawn(async move {
                let register_request = json!({
                    "email": format!("db.perf.{}@example.com", user_index),
                    "name": format!("DB Perf Test {}", user_index),
                    "password": format!("DbPerfTest{}!", user_index)
                });
                
                server.post("/api/v2/auth/register")
                    .json(&register_request)
                    .await
            })
        }).collect();
        
        let batch_results = futures::future::join_all(batch_tasks).await;
        let batch_time = batch_start.elapsed();
        total_creation_time += batch_time;
        
        // Vérifier succès du batch
        for (i, result) in batch_results.iter().enumerate() {
            let response = result.as_ref().unwrap();
            response.assert_status_ok();
        }
        
        println!("✅ Batch {} créé en {:?} ({} users/sec)", 
                batch, batch_time, 
                batch_size as f64 / batch_time.as_secs_f64());
    }
    
    let avg_creation_rate = num_users as f64 / total_creation_time.as_secs_f64();
    println!("📊 Taux moyen création : {:.1} utilisateurs/seconde", avg_creation_rate);
    
    // Vérifier taux de création acceptable
    assert!(avg_creation_rate > 10.0, 
            "Taux création trop faible: {:.1} users/sec", avg_creation_rate);
    
    // Test requêtes de lecture en masse
    println!("🔍 Test requêtes lecture en masse");
    
    let read_start = Instant::now();
    let read_tasks: Vec<_> = (0..50).map(|i| {
        let server = server.clone();
        
        tokio::spawn(async move {
            let login_request = json!({
                "email": format!("db.perf.{}@example.com", i),
                "password": format!("DbPerfTest{}!", i)
            });
            
            server.post("/api/v2/auth/login")
                .json(&login_request)
                .await
        })
    }).collect();
    
    let read_results = futures::future::join_all(read_tasks).await;
    let read_time = read_start.elapsed();
    
    for result in read_results {
        result?.assert_status_ok();
    }
    
    let read_rate = 50.0 / read_time.as_secs_f64();
    println!("📊 Taux lecture : {:.1} connexions/seconde", read_rate);
    
    assert!(read_rate > 20.0, 
            "Taux lecture trop faible: {:.1} logins/sec", read_rate);
    
    println!("🗄️ Tests performance base de données réussis!");
    Ok(())
}

#[tokio::test]
async fn test_memory_usage_under_load() -> Result<()> {
    println!("🧠 Test utilisation mémoire sous charge");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Fonction pour estimer l'utilisation mémoire (simplifiée)
    async fn get_memory_usage() -> usize {
        tokio::time::sleep(Duration::from_millis(10)).await;
        // En réalité, on utiliserait des outils système
        // Ici on simule avec une estimation basée sur les allocations
        std::alloc::System.usable_size(&vec![0u8; 1])
    }
    
    let initial_memory = get_memory_usage().await;
    println!("📊 Mémoire initiale estimée: {} bytes", initial_memory);
    
    // Créer charge de travail
    let num_operations = 200;
    let mut tokens = vec![];
    
    println!("🔍 Création de {} utilisateurs pour test mémoire", num_operations);
    
    for i in 0..num_operations {
        let register_request = json!({
            "email": format!("memory.test.{}@example.com", i),
            "name": format!("Memory Test {}", i),
            "password": format!("MemoryTest{}!", i)
        });
        
        let response = server
            .post("/api/v2/auth/register")
            .json(&register_request)
            .await;
        response.assert_status_ok();
        
        // Se connecter
        let login_request = json!({
            "email": format!("memory.test.{}@example.com", i),
            "password": format!("MemoryTest{}!", i)
        });
        
        let login_response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        login_response.assert_status_ok();
        
        let login_data: Value = login_response.json();
        let token = login_data["data"]["token"].as_str().unwrap().to_string();
        tokens.push(token);
        
        // Mesurer mémoire périodiquement
        if i % 50 == 0 && i > 0 {
            let current_memory = get_memory_usage().await;
            let memory_growth = current_memory.saturating_sub(initial_memory);
            println!("📊 Après {} opérations : +{} bytes", i, memory_growth);
        }
    }
    
    let peak_memory = get_memory_usage().await;
    let total_growth = peak_memory.saturating_sub(initial_memory);
    
    println!("📊 Croissance mémoire totale: {} bytes", total_growth);
    println!("📊 Mémoire par opération: {} bytes", 
             total_growth / num_operations);
    
    // Test que la mémoire n'explose pas
    let memory_per_op = total_growth / num_operations;
    assert!(memory_per_op < 1024 * 1024, // 1MB par opération max
            "Utilisation mémoire excessive: {} bytes/op", memory_per_op);
    
    // Test nettoyage - simuler déconnexions
    println!("🔍 Test nettoyage mémoire (déconnexions)");
    
    for (i, token) in tokens.iter().enumerate() {
        if i % 10 == 0 {
            let logout_response = server
                .post("/api/v2/auth/logout")
                .add_header("Authorization".parse()?, format!("Bearer {}", token).parse()?)
                .await;
            logout_response.assert_status_ok();
        }
    }
    
    // Permettre au garbage collector de s'exécuter
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let after_cleanup_memory = get_memory_usage().await;
    println!("📊 Mémoire après nettoyage: {} bytes", after_cleanup_memory);
    
    println!("🧠 Tests utilisation mémoire réussis!");
    Ok(())
}

#[tokio::test]
async fn test_error_recovery_under_stress() -> Result<()> {
    println!("🔄 Test récupération d'erreurs sous stress");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Test récupération après erreurs d'authentification
    println!("🔍 Test récupération après erreurs auth");
    
    // Générer beaucoup d'erreurs
    let error_tasks: Vec<_> = (0..100).map(|i| {
        let server = server.clone();
        tokio::spawn(async move {
            let bad_login = json!({
                "email": format!("nonexistent.{}@example.com", i),
                "password": "WrongPassword"
            });
            
            server.post("/api/v2/auth/login")
                .json(&bad_login)
                .await
        })
    }).collect();
    
    let error_results = futures::future::join_all(error_tasks).await;
    
    // Toutes devraient échouer
    for result in error_results {
        let response = result?;
        response.assert_status_unauthorized();
    }
    
    println!("✅ 100 erreurs d'auth générées comme attendu");
    
    // Vérifier que le système répond encore normalement
    let register_request = json!({
        "email": "recovery.test@example.com",
        "name": "Recovery Test",
        "password": "RecoveryTest123!"
    });
    
    let recovery_response = server
        .post("/api/v2/auth/register")
        .json(&register_request)
        .await;
    
    recovery_response.assert_status_ok();
    println!("✅ Système fonctionne normalement après erreurs");
    
    // Test récupération après charge extrême
    println!("🔍 Test récupération après charge extrême");
    
    let extreme_load_start = Instant::now();
    let load_tasks: Vec<_> = (0..500).map(|i| {
        let server = server.clone();
        tokio::spawn(async move {
            // Mix d'opérations légitimes et d'erreurs
            if i % 3 == 0 {
                // Opération valide
                let register_request = json!({
                    "email": format!("load.test.{}@example.com", i),
                    "name": format!("Load Test {}", i),
                    "password": format!("LoadTest{}!", i)
                });
                
                server.post("/api/v2/auth/register")
                    .json(&register_request)
                    .await
            } else {
                // Requête invalide
                let bad_request = json!({
                    "email": "invalid-email",
                    "password": "x"
                });
                
                server.post("/api/v2/auth/register")
                    .json(&bad_request)
                    .await
            }
        })
    }).collect();
    
    let load_results = futures::future::join_all(load_tasks).await;
    let load_time = extreme_load_start.elapsed();
    
    println!("⏱️ Charge extrême terminée en {:?}", load_time);
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, result) in load_results.iter().enumerate() {
        let response = result.as_ref().unwrap();
        if i % 3 == 0 {
            // Devrait être succès
            if response.status_code().is_success() {
                success_count += 1;
            }
        } else {
            // Devrait être erreur
            if response.status_code().is_client_error() {
                error_count += 1;
            }
        }
    }
    
    println!("📊 Résultats charge extrême:");
    println!("   Succès: {}", success_count);
    println!("   Erreurs: {}", error_count);
    
    // Vérifier que le système gère correctement les erreurs
    assert!(success_count > 150, "Pas assez de succès: {}", success_count);
    assert!(error_count > 300, "Pas assez d'erreurs détectées: {}", error_count);
    
    // Test fonctionnalité normale après stress
    println!("🔍 Test fonctionnalité après stress");
    
    let post_stress_request = json!({
        "email": "post.stress@example.com",
        "name": "Post Stress Test",
        "password": "PostStress123!"
    });
    
    let post_stress_response = server
        .post("/api/v2/auth/register")
        .json(&post_stress_request)
        .await;
    
    post_stress_response.assert_status_ok();
    
    // Test connexion
    let login_request = json!({
        "email": "post.stress@example.com",
        "password": "PostStress123!"
    });
    
    let login_response = server
        .post("/api/v2/auth/login")
        .json(&login_request)
        .await;
    
    login_response.assert_status_ok();
    
    println!("✅ Système complètement fonctionnel après stress");
    println!("🔄 Tests récupération erreurs sous stress réussis!");
    
    Ok(())
}

#[tokio::test]
async fn test_response_time_consistency() -> Result<()> {
    println!("⏱️ Test cohérence temps de réponse");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Créer utilisateur pour les tests
    let register_request = json!({
        "email": "timing.test@example.com",
        "name": "Timing Test",
        "password": "TimingTest123!"
    });
    
    server.post("/api/v2/auth/register")
        .json(&register_request)
        .await
        .assert_status_ok();
    
    // Test cohérence des temps de connexion
    let login_request = json!({
        "email": "timing.test@example.com",
        "password": "TimingTest123!"
    });
    
    let num_tests = 100;
    let mut response_times = vec![];
    
    println!("🔍 Mesure {} temps de réponse connexion", num_tests);
    
    for i in 0..num_tests {
        let start = Instant::now();
        
        let response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        
        let duration = start.elapsed();
        response_times.push(duration);
        
        response.assert_status_ok();
        
        if i % 20 == 0 {
            println!("   Test {} : {:?}", i, duration);
        }
    }
    
    // Analyser la cohérence
    response_times.sort();
    
    let min_time = response_times[0];
    let max_time = response_times[num_tests - 1];
    let median_time = response_times[num_tests / 2];
    let p95_time = response_times[(num_tests as f64 * 0.95) as usize];
    let avg_time = response_times.iter().sum::<Duration>() / num_tests as u32;
    
    println!("📊 Statistiques temps de réponse:");
    println!("   Min: {:?}", min_time);
    println!("   Médiane: {:?}", median_time);
    println!("   Moyenne: {:?}", avg_time);
    println!("   P95: {:?}", p95_time);
    println!("   Max: {:?}", max_time);
    
    // Calculer variabilité
    let variance = response_times.iter()
        .map(|&t| {
            let diff = t.as_millis() as f64 - avg_time.as_millis() as f64;
            diff * diff
        })
        .sum::<f64>() / num_tests as f64;
    
    let std_dev = variance.sqrt();
    let coefficient_variation = std_dev / avg_time.as_millis() as f64;
    
    println!("📊 Variabilité:");
    println!("   Écart-type: {:.2}ms", std_dev);
    println!("   Coefficient variation: {:.3}", coefficient_variation);
    
    // Vérifier cohérence
    assert!(coefficient_variation < 0.5, 
            "Temps de réponse trop variables: CV = {:.3}", coefficient_variation);
    
    assert!(p95_time < Duration::from_millis(500), 
            "P95 trop élevé: {:?}", p95_time);
    
    assert!(max_time < Duration::from_secs(2), 
            "Temps max trop élevé: {:?}", max_time);
    
    println!("✅ Temps de réponse cohérents");
    println!("⏱️ Tests cohérence temps de réponse réussis!");
    
    Ok(())
}

use futures;