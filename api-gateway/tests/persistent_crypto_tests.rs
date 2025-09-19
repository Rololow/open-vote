use anyhow::Result;
use serde_json::{json, Value};
use api_gateway::persistent_gateway::PersistentApiGateway;
use api_gateway::crypto_service::CryptographicService;
use api_gateway::persistent_database::DatabaseService;
use axum_test::TestServer;
use crypto_lib::KeyPair;
use uuid::Uuid;

/// Tests de sécurité cryptographique approfondis
/// 
/// Ces tests vérifient la robustesse de la cryptographie:
/// - Génération et stockage sécurisé des clés
/// - Chiffrement/déchiffrement
/// - Signatures et vérifications
/// - Protection contre les attaques

#[tokio::test]
async fn test_cryptographic_key_generation_security() -> Result<()> {
    println!("🔐 Test sécurité génération de clés");
    
    let db = DatabaseService::new("sqlite::memory:").await?;
    let crypto_service = CryptographicService::new(db.clone());
    
    // Créer plusieurs utilisateurs pour tester l'unicité des clés
    let mut user_ids = vec![];
    let mut public_keys = vec![];
    
    for i in 0..10 {
        let email = format!("crypto.test.{}@example.com", i);
        let name = format!("Crypto Test User {}", i);
        let password_hash = "dummy_hash"; // En vrai test, utiliser Argon2
        
        let user = db.create_user(&email, &name, password_hash).await?;
        user_ids.push(user.id);
        
        // Simuler validation d'identité pour permettre génération clés
        db.update_user_identity_status(&user.id, api_gateway::models::IdentityStatus::Validated).await?;
    }
    
    println!("✅ {} utilisateurs créés", user_ids.len());
    
    // Test génération clés pour chaque utilisateur
    for (i, user_id) in user_ids.iter().enumerate() {
        let password = format!("TestPassword{}!", i);
        
        // Générer les clés
        let crypto_keys = crypto_service.generate_keys_for_user(user_id, &password).await?;
        public_keys.push(crypto_keys.public_key.clone());
        
        println!("✅ Clés générées pour utilisateur {}", i);
        
        // Vérifier récupération
        let retrieved_keypair = crypto_service.get_user_keys(user_id, &password).await?;
        assert_eq!(retrieved_keypair.public_key(), &crypto_keys.public_key);
        
        println!("✅ Clés récupérées et vérifiées pour utilisateur {}", i);
    }
    
    // Vérifier unicité des clés publiques
    for i in 0..public_keys.len() {
        for j in (i+1)..public_keys.len() {
            assert_ne!(public_keys[i], public_keys[j], 
                      "Clés publiques identiques entre utilisateurs {} et {}", i, j);
        }
    }
    
    println!("✅ Toutes les clés publiques sont uniques");
    
    // Test résistance aux mauvais mots de passe
    let user_id = &user_ids[0];
    let wrong_password = "WrongPassword123!";
    
    let result = crypto_service.get_user_keys(user_id, wrong_password).await;
    assert!(result.is_err(), "Déchiffrement devrait échouer avec mauvais mot de passe");
    
    println!("✅ Protection contre mauvais mots de passe fonctionne");
    println!("🔐 Tests sécurité génération clés réussis!");
    
    Ok(())
}

#[tokio::test]
async fn test_signature_verification_security() -> Result<()> {
    println!("✍️ Test sécurité signatures cryptographiques");
    
    let db = DatabaseService::new("sqlite::memory:").await?;
    let crypto_service = CryptographicService::new(db.clone());
    
    // Créer utilisateur
    let user = db.create_user("sig.test@example.com", "Signature Tester", "hash").await?;
    db.update_user_identity_status(&user.id, api_gateway::models::IdentityStatus::Validated).await?;
    
    let password = "SignatureTestPassword123!";
    crypto_service.generate_keys_for_user(&user.id, password).await?;
    
    // Test messages de différentes tailles
    let test_messages = vec![
        b"Court message".to_vec(),
        b"Message de taille moyenne pour tester la signature".to_vec(),
        (0..1000).map(|i| (i % 256) as u8).collect::<Vec<u8>>(), // Message de 1KB
        (0..10000).map(|i| (i % 256) as u8).collect::<Vec<u8>>(), // Message de 10KB
    ];
    
    for (i, message) in test_messages.iter().enumerate() {
        println!("🔍 Test signature message {} ({} bytes)", i, message.len());
        
        // Signer le message
        let signature = crypto_service.sign_with_user_keys(&user.id, password, message).await?;
        
        // Vérifier signature
        let is_valid = crypto_service.verify_user_signature(&user.id, message, &signature).await?;
        assert!(is_valid, "Signature devrait être valide pour message {}", i);
        
        // Test modification du message
        let mut modified_message = message.clone();
        if !modified_message.is_empty() {
            modified_message[0] = modified_message[0].wrapping_add(1);
        }
        
        let is_invalid = crypto_service.verify_user_signature(&user.id, &modified_message, &signature).await?;
        assert!(!is_invalid, "Signature devrait être invalide pour message modifié {}", i);
        
        println!("✅ Signature et vérification OK pour message {}", i);
    }
    
    // Test signatures avec différents utilisateurs
    println!("🔍 Test cross-utilisateur");
    
    let user2 = db.create_user("sig.test2@example.com", "Signature Tester 2", "hash").await?;
    db.update_user_identity_status(&user2.id, api_gateway::models::IdentityStatus::Validated).await?;
    crypto_service.generate_keys_for_user(&user2.id, password).await?;
    
    let message = b"Message cross-utilisateur";
    let signature_user1 = crypto_service.sign_with_user_keys(&user.id, password, message).await?;
    
    // Signature de user1 ne devrait pas être valide pour user2
    let cross_valid = crypto_service.verify_user_signature(&user2.id, message, &signature_user1).await?;
    assert!(!cross_valid, "Signature cross-utilisateur devrait être invalide");
    
    println!("✅ Protection cross-utilisateur fonctionne");
    println!("✍️ Tests sécurité signatures réussis!");
    
    Ok(())
}

#[tokio::test]
async fn test_password_strength_and_protection() -> Result<()> {
    println!("🛡️ Test force et protection des mots de passe");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Test mots de passe faibles (doivent être rejetés)
    let weak_passwords = vec![
        "123",           // Trop court
        "password",      // Pas de majuscule/chiffre
        "PASSWORD",      // Pas de minuscule/chiffre
        "12345678",      // Que des chiffres
        "abcdefgh",      // Que des minuscules
        "ABCDEFGH",      // Que des majuscules
        "Password",      // Pas de chiffre
        "password1",     // Pas de majuscule
        "PASSWORD1",     // Pas de minuscule
    ];
    
    for (i, weak_password) in weak_passwords.iter().enumerate() {
        println!("🔍 Test mot de passe faible {}: '{}'", i, weak_password);
        
        let register_request = json!({
            "email": format!("weak.password.{}@example.com", i),
            "name": format!("Weak Password Test {}", i),
            "password": weak_password
        });
        
        let response = server
            .post("/api/v2/auth/register")
            .json(&register_request)
            .await;
        
        response.assert_status_bad_request();
        let error_data: Value = response.json();
        assert_eq!(error_data["success"], false);
        assert!(error_data["error"].as_str().unwrap().contains("mot de passe"));
        
        println!("✅ Mot de passe faible {} correctement rejeté", i);
    }
    
    // Test mots de passe forts (doivent être acceptés)
    let strong_passwords = vec![
        "StrongPass123!",
        "MySecure2024#",
        "Complex9Password$",
        "Unbreakable7Key&",
    ];
    
    for (i, strong_password) in strong_passwords.iter().enumerate() {
        println!("🔍 Test mot de passe fort {}", i);
        
        let register_request = json!({
            "email": format!("strong.password.{}@example.com", i),
            "name": format!("Strong Password Test {}", i),
            "password": strong_password
        });
        
        let response = server
            .post("/api/v2/auth/register")
            .json(&register_request)
            .await;
        
        response.assert_status_ok();
        let success_data: Value = response.json();
        assert_eq!(success_data["success"], true);
        
        println!("✅ Mot de passe fort {} accepté", i);
        
        // Vérifier connexion avec mot de passe fort
        let login_request = json!({
            "email": format!("strong.password.{}@example.com", i),
            "password": strong_password
        });
        
        let login_response = server
            .post("/api/v2/auth/login")
            .json(&login_request)
            .await;
        
        login_response.assert_status_ok();
        println!("✅ Connexion avec mot de passe fort {} réussie", i);
    }
    
    println!("🛡️ Tests force mots de passe réussis!");
    Ok(())
}

#[tokio::test]
async fn test_session_security() -> Result<()> {
    println!("🎫 Test sécurité des sessions");
    
    let gateway = PersistentApiGateway::test_instance().await?;
    let server = TestServer::new(gateway.create_router())?;
    
    // Créer utilisateur
    let register_request = json!({
        "email": "session.test@example.com",
        "name": "Session Tester",
        "password": "SessionTestPassword123!"
    });
    
    server.post("/api/v2/auth/register")
        .json(&register_request)
        .await
        .assert_status_ok();
    
    // Connexion pour obtenir token
    let login_request = json!({
        "email": "session.test@example.com",
        "password": "SessionTestPassword123!"
    });
    
    let login_response = server
        .post("/api/v2/auth/login")
        .json(&login_request)
        .await;
    
    login_response.assert_status_ok();
    let login_data: Value = login_response.json();
    let token = login_data["data"]["token"].as_str().unwrap();
    
    println!("✅ Token obtenu");
    
    // Test accès avec token valide
    let profile_response = server
        .get("/api/v2/auth/profile")
        .add_header("Authorization".parse()?, format!("Bearer {}", token).parse()?)
        .await;
    
    profile_response.assert_status_ok();
    println!("✅ Accès avec token valide réussi");
    
    // Test accès sans token
    let no_token_response = server
        .get("/api/v2/auth/profile")
        .await;
    
    no_token_response.assert_status_unauthorized();
    println!("✅ Accès sans token correctement bloqué");
    
    // Test token malformé
    let malformed_response = server
        .get("/api/v2/auth/profile")
        .add_header("Authorization".parse()?, "Bearer invalid.token.here".parse()?)
        .await;
    
    malformed_response.assert_status_unauthorized();
    println!("✅ Token malformé correctement rejeté");
    
    // Test déconnexion
    let logout_response = server
        .post("/api/v2/auth/logout")
        .add_header("Authorization".parse()?, format!("Bearer {}", token).parse()?)
        .await;
    
    logout_response.assert_status_ok();
    println!("✅ Déconnexion réussie");
    
    // Test accès après déconnexion
    let after_logout_response = server
        .get("/api/v2/auth/profile")
        .add_header("Authorization".parse()?, format!("Bearer {}", token).parse()?)
        .await;
    
    after_logout_response.assert_status_unauthorized();
    println!("✅ Token révoqué après déconnexion");
    
    println!("🎫 Tests sécurité sessions réussis!");
    Ok(())
}

#[tokio::test]
async fn test_cryptographic_randomness() -> Result<()> {
    println!("🎲 Test qualité de l'aléa cryptographique");
    
    let db = DatabaseService::new("sqlite::memory:").await?;
    let crypto_service = CryptographicService::new(db.clone());
    
    // Générer plusieurs utilisateurs et analyser l'aléa de leurs clés
    let num_users = 20;
    let mut public_keys = vec![];
    let mut private_key_hashes = vec![];
    
    for i in 0..num_users {
        let user = db.create_user(
            &format!("random.test.{}@example.com", i),
            &format!("Random Test {}", i),
            "hash"
        ).await?;
        
        db.update_user_identity_status(&user.id, api_gateway::models::IdentityStatus::Validated).await?;
        
        let password = format!("RandomTest{}!", i);
        let crypto_keys = crypto_service.generate_keys_for_user(&user.id, &password).await?;
        
        public_keys.push(crypto_keys.public_key.clone());
        
        // Récupérer et hasher la clé privée pour test d'unicité
        let keypair = crypto_service.get_user_keys(&user.id, &password).await?;
        let private_key_hash = sha2::Sha256::digest(keypair.private_key());
        private_key_hashes.push(private_key_hash.to_vec());
    }
    
    // Vérifier unicité des clés publiques
    for i in 0..public_keys.len() {
        for j in (i+1)..public_keys.len() {
            assert_ne!(public_keys[i], public_keys[j], 
                      "Clés publiques identiques détectées");
        }
    }
    
    // Vérifier unicité des clés privées (via hash)
    for i in 0..private_key_hashes.len() {
        for j in (i+1)..private_key_hashes.len() {
            assert_ne!(private_key_hashes[i], private_key_hashes[j], 
                      "Clés privées identiques détectées");
        }
    }
    
    // Test de distribution des bits (simple)
    println!("🔍 Analyse distribution des clés publiques");
    
    let mut bit_counts = vec![0; 8];
    for public_key in &public_keys {
        for byte in public_key {
            for bit in 0..8 {
                if (byte >> bit) & 1 == 1 {
                    bit_counts[bit] += 1;
                }
            }
        }
    }
    
    let total_bits = public_keys.len() * 32 * 8; // 32 bytes * 8 bits
    let expected_ones = total_bits / 2;
    
    for (bit_pos, count) in bit_counts.iter().enumerate() {
        let ratio = *count as f64 / (total_bits / 8) as f64;
        println!("   Bit {}: {:.3} (attendu ~0.5)", bit_pos, ratio);
        
        // Vérifier que la distribution n'est pas trop biaisée
        assert!(ratio > 0.4 && ratio < 0.6, 
                "Distribution bit {} trop biaisée: {:.3}", bit_pos, ratio);
    }
    
    println!("✅ {} clés générées avec bonne entropie", num_users);
    println!("🎲 Tests qualité aléa cryptographique réussis!");
    
    Ok(())
}

#[tokio::test]
async fn test_key_backup_and_restore_security() -> Result<()> {
    println!("💾 Test sécurité backup/restore des clés");
    
    let db = DatabaseService::new("sqlite::memory:").await?;
    let crypto_service = CryptographicService::new(db.clone());
    
    // Créer utilisateur avec clés
    let user = db.create_user("backup.test@example.com", "Backup Tester", "hash").await?;
    db.update_user_identity_status(&user.id, api_gateway::models::IdentityStatus::Validated).await?;
    
    let password = "BackupTestPassword123!";
    let original_keys = crypto_service.generate_keys_for_user(&user.id, password).await?;
    
    println!("✅ Clés originales générées");
    
    // Test signature avec clés originales
    let test_message = b"Message de test pour backup";
    let original_signature = crypto_service.sign_with_user_keys(&user.id, password, test_message).await?;
    
    // Créer backup
    let backup_data = crypto_service.backup_user_keys(&user.id, password).await?;
    println!("✅ Backup créé");
    
    // Vérifier que le backup est chiffré (ne contient pas de données en clair)
    assert!(!backup_data.contains("backup.test@example.com"), 
            "Backup ne devrait pas contenir email en clair");
    assert!(!backup_data.contains("BackupTestPassword123!"), 
            "Backup ne devrait pas contenir mot de passe en clair");
    
    // Simuler suppression des clés (dans un vrai système)
    // Pour ce test, on va créer un nouvel utilisateur
    let restore_user = db.create_user("restore.test@example.com", "Restore Tester", "hash").await?;
    db.update_user_identity_status(&restore_user.id, api_gateway::models::IdentityStatus::Validated).await?;
    
    // Restaurer les clés
    crypto_service.restore_user_keys(&restore_user.id, &backup_data, password).await?;
    println!("✅ Clés restaurées");
    
    // Vérifier que les clés restaurées fonctionnent
    let restored_signature = crypto_service.sign_with_user_keys(&restore_user.id, password, test_message).await?;
    
    // Les signatures devraient être identiques car même clé privée
    assert_eq!(original_signature, restored_signature, 
               "Signatures devraient être identiques après restore");
    
    // Test avec mauvais mot de passe pour restore
    let wrong_password = "WrongBackupPassword123!";
    let user3 = db.create_user("restore.fail@example.com", "Restore Fail", "hash").await?;
    db.update_user_identity_status(&user3.id, api_gateway::models::IdentityStatus::Validated).await?;
    
    let restore_result = crypto_service.restore_user_keys(&user3.id, &backup_data, wrong_password).await;
    assert!(restore_result.is_err(), "Restore devrait échouer avec mauvais mot de passe");
    
    println!("✅ Protection backup contre mauvais mot de passe fonctionne");
    println!("💾 Tests sécurité backup/restore réussis!");
    
    Ok(())
}

use sha2::{Sha256, Digest};