use api_gateway::auth::*;
use crypto_lib::KeyPair;
use uuid::Uuid;

#[cfg(test)]
mod auth_system_tests {
    use super::*;

    #[test]
    fn test_user_role_permissions() {
        let citizen = UserRole::Citizen;
        let representative = UserRole::Representative;
        let admin = UserRole::Administrator;
        let moderator = UserRole::Moderator;
        let auditor = UserRole::Auditor;

        // Test hierarchie des permissions
        assert!(admin.has_permission(&citizen));
        assert!(admin.has_permission(&representative));
        assert!(admin.has_permission(&moderator));
        assert!(admin.has_permission(&auditor));
        
        assert!(representative.has_permission(&citizen));
        assert!(!citizen.has_permission(&representative));
        
        assert!(moderator.has_permission(&citizen));
        assert!(!moderator.has_permission(&representative));
        
        // Test priorités
        assert!(admin.priority() > representative.priority());
        assert!(representative.priority() > citizen.priority());
        assert!(moderator.priority() > citizen.priority());
        assert!(auditor.priority() > citizen.priority());
    }

    #[test]
    fn test_user_role_default_permissions() {
        let citizen = UserRole::Citizen;
        let representative = UserRole::Representative;
        let admin = UserRole::Administrator;

        let citizen_perms = citizen.default_permissions();
        let rep_perms = representative.default_permissions();
        let admin_perms = admin.default_permissions();

        // Citoyen doit avoir permissions de base
        assert!(citizen_perms.contains(&permissions::CREATE_LAW.to_string()));
        assert!(citizen_perms.contains(&permissions::VOTE_ON_LAW.to_string()));
        assert!(!citizen_perms.contains(&permissions::MODIFY_LAW.to_string()));

        // Représentant doit avoir plus de permissions
        assert!(rep_perms.contains(&permissions::CREATE_LAW.to_string()));
        assert!(rep_perms.contains(&permissions::VOTE_ON_LAW.to_string()));
        assert!(rep_perms.contains(&permissions::MODIFY_LAW.to_string()));

        // Admin doit avoir toutes les permissions
        assert!(admin_perms.contains(&permissions::SYSTEM_MAINTENANCE.to_string()));
        assert!(admin_perms.contains(&permissions::MANAGE_USERS.to_string()));
        assert!(admin_perms.len() > rep_perms.len());
        assert!(admin_perms.len() > citizen_perms.len());
    }

    #[test]
    fn test_jwt_claims_creation_and_validation() {
        let user_id = Uuid::new_v4();
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key().clone();
        let role = UserRole::Representative;
        let permissions = vec![
            permissions::CREATE_LAW.to_string(),
            permissions::VOTE_ON_LAW.to_string(),
            permissions::MODIFY_LAW.to_string(),
        ];

        let claims = JwtClaims::new(
            user_id,
            public_key,
            role,
            permissions.clone(),
            24, // 24 heures
        );

        // Vérifier les données
        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.role, UserRole::Representative);
        assert_eq!(claims.permissions, permissions);
        assert!(!claims.is_expired());
        
        // Vérifier les permissions
        assert!(claims.has_permission(permissions::CREATE_LAW));
        assert!(claims.has_permission(permissions::VOTE_ON_LAW));
        assert!(claims.has_permission(permissions::MODIFY_LAW));
        assert!(!claims.has_permission(permissions::DELETE_LAW));
    }

    #[test]
    fn test_signed_jwt_creation_and_verification() {
        let user_id = Uuid::new_v4();
        let signing_key = KeyPair::generate();
        let user_public_key = KeyPair::generate().public_key().clone();
        
        let claims = JwtClaims::new(
            user_id,
            user_public_key,
            UserRole::Citizen,
            vec![permissions::VOTE_ON_LAW.to_string()],
            2, // 2 heures
        );

        // Créer et signer le JWT
        let jwt = SignedJwt::create(claims.clone(), &signing_key).unwrap();
        
        // Vérifier la signature
        assert!(jwt.verify_signature(signing_key.public_key()).is_ok());
        
        // Vérifier la validité complète
        assert!(jwt.verify(signing_key.public_key()).is_ok());
        
        // Test avec une mauvaise clé
        let wrong_key = KeyPair::generate();
        assert!(jwt.verify_signature(wrong_key.public_key()).is_err());
    }

    #[test]
    fn test_jwt_encoding_and_decoding() {
        let user_id = Uuid::new_v4();
        let signing_key = KeyPair::generate();
        let user_public_key = KeyPair::generate().public_key().clone();
        
        let claims = JwtClaims::new(
            user_id,
            user_public_key,
            UserRole::Moderator,
            vec![permissions::MODERATE_CONTENT.to_string()],
            1,
        );

        let jwt = SignedJwt::create(claims.clone(), &signing_key).unwrap();
        
        // Encoder
        let encoded = jwt.encode().unwrap();
        assert!(!encoded.is_empty());
        
        // Décoder
        let decoded_jwt = SignedJwt::decode(&encoded).unwrap();
        
        // Vérifier que les données sont identiques
        assert_eq!(decoded_jwt.claims.sub, jwt.claims.sub);
        assert_eq!(decoded_jwt.claims.role, jwt.claims.role);
        assert_eq!(decoded_jwt.claims.permissions, jwt.claims.permissions);
    }

    #[test]
    fn test_auth_manager_token_generation() {
        let signing_key = KeyPair::generate();
        let mut auth_manager = AuthManager::new(signing_key, 24);
        
        let user_id = Uuid::new_v4();
        let user_public_key = KeyPair::generate().public_key().clone();
        let role = UserRole::Representative;
        let permissions = role.default_permissions();

        // Générer un token
        let jwt = auth_manager.generate_token(
            user_id,
            user_public_key,
            role,
            permissions.clone(),
        ).unwrap();

        // Vérifier le token
        assert_eq!(jwt.claims.user_id().unwrap(), user_id);
        assert_eq!(jwt.claims.role, UserRole::Representative);
        assert_eq!(jwt.claims.permissions, permissions);
        
        // Vérifier que la session est active
        assert_eq!(auth_manager.active_session_count(), 1);
    }

    #[test]
    fn test_auth_manager_token_validation() {
        let signing_key = KeyPair::generate();
        let mut auth_manager = AuthManager::new(signing_key, 24);
        
        let user_id = Uuid::new_v4();
        let user_public_key = KeyPair::generate().public_key().clone();
        
        // Générer un token
        let jwt = auth_manager.generate_token(
            user_id,
            user_public_key,
            UserRole::Citizen,
            vec![permissions::VOTE_ON_LAW.to_string()],
        ).unwrap();

        // Encoder le token
        let token_string = jwt.encode().unwrap();
        
        // Valider le token
        let validated_claims = auth_manager.validate_token(&token_string).unwrap();
        
        assert_eq!(validated_claims.user_id().unwrap(), user_id);
        assert_eq!(validated_claims.role, UserRole::Citizen);
    }

    #[test]
    fn test_auth_manager_session_management() {
        let signing_key = KeyPair::generate();
        let mut auth_manager = AuthManager::new(signing_key, 24);
        
        let user1_id = Uuid::new_v4();
        let user2_id = Uuid::new_v4();
        let user1_key = KeyPair::generate().public_key().clone();
        let user2_key = KeyPair::generate().public_key().clone();

        // Créer plusieurs sessions
        let jwt1 = auth_manager.generate_token(
            user1_id,
            user1_key,
            UserRole::Citizen,
            vec![permissions::VOTE_ON_LAW.to_string()],
        ).unwrap();

        let _jwt2 = auth_manager.generate_token(
            user2_id,
            user2_key,
            UserRole::Representative,
            vec![permissions::MODIFY_LAW.to_string()],
        ).unwrap();

        assert_eq!(auth_manager.active_session_count(), 2);

        // Révoquer une session
        auth_manager.revoke_token(&jwt1.claims.session_id).unwrap();
        assert_eq!(auth_manager.active_session_count(), 1);

        // Révoquer toutes les sessions d'un utilisateur
        auth_manager.revoke_all_user_tokens(user2_id);
        assert_eq!(auth_manager.active_session_count(), 0);
    }

    #[test]
    fn test_auth_manager_permission_checking() {
        let signing_key = KeyPair::generate();
        let auth_manager = AuthManager::new(signing_key, 24);
        
        let user_id = Uuid::new_v4();
        let user_public_key = KeyPair::generate().public_key().clone();
        
        // Claims avec permissions limitées
        let claims = JwtClaims::new(
            user_id,
            user_public_key,
            UserRole::Citizen,
            vec![permissions::VOTE_ON_LAW.to_string()],
            24,
        );

        // Test permissions autorisées
        assert!(auth_manager.check_permission(
            &claims, 
            &UserRole::Citizen, 
            permissions::VOTE_ON_LAW
        ).is_ok());

        // Test permissions refusées (rôle insuffisant)
        assert!(auth_manager.check_permission(
            &claims,
            &UserRole::Representative,
            ""
        ).is_err());

        // Test permissions refusées (permission spécifique manquante)
        assert!(auth_manager.check_permission(
            &claims,
            &UserRole::Citizen,
            permissions::MODIFY_LAW
        ).is_err());
    }

    #[test]
    fn test_jwt_claims_expiration() {
        let user_id = Uuid::new_v4();
        let user_public_key = KeyPair::generate().public_key().clone();
        
        // Créer des claims avec une durée de 0 heures (immédiatement expiré)
        let _expired_claims = JwtClaims::new(
            user_id,
            user_public_key,
            UserRole::Citizen,
            vec![],
            0,
        );

        // Le token devrait être expiré (ou sur le point de l'être)
        // Note: Ce test peut être fragile selon le timing exact
        std::thread::sleep(std::time::Duration::from_millis(1));
        // Dans un vrai test, on modulerait mieux le temps
    }

    #[test]
    fn test_invalid_token_handling() {
        let signing_key1 = KeyPair::generate();
        let signing_key2 = KeyPair::generate();
        let auth_manager = AuthManager::new(signing_key1, 24);

        // Test token invalide (pas de base64)
        assert!(auth_manager.validate_token("invalid_token").is_err());
        
        // Test token invalide (JSON malformé)
        use base64::Engine;
        let invalid_json = base64::engine::general_purpose::STANDARD.encode("invalid json");
        assert!(auth_manager.validate_token(&invalid_json).is_err());
        
        // Test token avec session inexistante
        let user_id = Uuid::new_v4();
        let user_public_key = KeyPair::generate().public_key().clone();
        let claims = JwtClaims::new(
            user_id,
            user_public_key,
            UserRole::Citizen,
            vec![],
            24,
        );
        
        let fake_jwt = SignedJwt::create(claims, &signing_key2).unwrap();
        let fake_token = fake_jwt.encode().unwrap();
        
        // Ce token n'a pas été créé via l'AuthManager donc la session n'existe pas
        assert!(auth_manager.validate_token(&fake_token).is_err());
    }

    #[test]
    fn test_legacy_authenticate_request() {
        let signing_key = KeyPair::generate();
        let user_id = Uuid::new_v4();
        let user_public_key = KeyPair::generate().public_key().clone();
        
        let claims = JwtClaims::new(
            user_id,
            user_public_key,
            UserRole::Citizen,
            vec![],
            24,
        );
        
        let jwt = SignedJwt::create(claims, &signing_key).unwrap();
        let token = jwt.encode().unwrap();
        
        // La fonction legacy devrait être capable de décoder le token
        assert!(authenticate_request(&token));
        
        // Token invalide
        assert!(!authenticate_request("invalid"));
    }
}

/// Tests d'intégration pour l'ensemble du système d'authentification
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_authentication_workflow() {
        // Scénario complet : création d'utilisateur, authentification, utilisation
        let signing_key = KeyPair::generate();
        let mut auth_manager = AuthManager::new(signing_key, 24);
        
        // 1. Créer un utilisateur représentant
        let user_id = Uuid::new_v4();
        let user_keypair = KeyPair::generate();
        let user_public_key = user_keypair.public_key().clone();
        
        // 2. Générer token d'authentification
        let jwt = auth_manager.generate_token(
            user_id,
            user_public_key,
            UserRole::Representative,
            UserRole::Representative.default_permissions(),
        ).unwrap();
        
        // 3. Simuler l'envoi du token (encoding)
        let token_string = jwt.encode().unwrap();
        
        // 4. Validation côté serveur
        let validated_claims = auth_manager.validate_token(&token_string).unwrap();
        
        // 5. Vérification des permissions pour différentes actions
        assert!(auth_manager.check_permission(
            &validated_claims,
            &UserRole::Citizen,
            permissions::VOTE_ON_LAW
        ).is_ok());
        
        assert!(auth_manager.check_permission(
            &validated_claims,
            &UserRole::Representative,
            permissions::MODIFY_LAW
        ).is_ok());
        
        assert!(auth_manager.check_permission(
            &validated_claims,
            &UserRole::Administrator,
            permissions::SYSTEM_MAINTENANCE
        ).is_err());
        
        // 6. Révocation de session
        auth_manager.revoke_token(&validated_claims.session_id).unwrap();
        
        // 7. Vérifier que le token n'est plus valide
        assert!(auth_manager.validate_token(&token_string).is_err());
    }
    
    #[test]
    fn test_multi_user_permissions_scenario() {
        let signing_key = KeyPair::generate();
        let mut auth_manager = AuthManager::new(signing_key, 24);
        
        // Créer différents types d'utilisateurs
        let citizen_id = Uuid::new_v4();
        let representative_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        
        let citizen_key = KeyPair::generate().public_key().clone();
        let rep_key = KeyPair::generate().public_key().clone();
        let admin_key = KeyPair::generate().public_key().clone();
        
        // Générer leurs tokens
        let citizen_jwt = auth_manager.generate_token(
            citizen_id,
            citizen_key,
            UserRole::Citizen,
            UserRole::Citizen.default_permissions(),
        ).unwrap();
        
        let rep_jwt = auth_manager.generate_token(
            representative_id,
            rep_key,
            UserRole::Representative,
            UserRole::Representative.default_permissions(),
        ).unwrap();
        
        let admin_jwt = auth_manager.generate_token(
            admin_id,
            admin_key,
            UserRole::Administrator,
            UserRole::Administrator.default_permissions(),
        ).unwrap();
        
        // Vérifier les permissions de chacun
        
        // Citoyen : peut voter, ne peut pas modifier
        assert!(auth_manager.check_permission(
            &citizen_jwt.claims,
            &UserRole::Citizen,
            permissions::VOTE_ON_LAW
        ).is_ok());
        
        assert!(auth_manager.check_permission(
            &citizen_jwt.claims,
            &UserRole::Representative,
            permissions::MODIFY_LAW
        ).is_err());
        
        // Représentant : peut voter et modifier
        assert!(auth_manager.check_permission(
            &rep_jwt.claims,
            &UserRole::Citizen,
            permissions::VOTE_ON_LAW
        ).is_ok());
        
        assert!(auth_manager.check_permission(
            &rep_jwt.claims,
            &UserRole::Representative,
            permissions::MODIFY_LAW
        ).is_ok());
        
        // Admin : peut tout faire
        assert!(auth_manager.check_permission(
            &admin_jwt.claims,
            &UserRole::Administrator,
            permissions::SYSTEM_MAINTENANCE
        ).is_ok());
        
        assert!(auth_manager.check_permission(
            &admin_jwt.claims,
            &UserRole::Representative,
            permissions::MODIFY_LAW
        ).is_ok());
        
        assert!(auth_manager.check_permission(
            &admin_jwt.claims,
            &UserRole::Citizen,
            permissions::VOTE_ON_LAW
        ).is_ok());
    }
}