use crypto_lib::{KeyPair, PublicKey, PrivateKey, Hash};

/// Tests pour le module de gestion des paires de clés cryptographiques
mod keypair_tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let message = b"test message";
        
        // Test signature et vérification
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature).is_ok());
        
        // Test avec mauvais message
        let bad_message = b"wrong message";
        assert!(keypair.verify(bad_message, &signature).is_err());
    }

    #[test]
    fn test_public_key_serialization() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();
        
        // Test conversion hex
        let hex = public_key.to_hex();
        let recovered = PublicKey::from_hex(&hex).unwrap();
        assert_eq!(public_key, &recovered);
        
        // Test conversion bytes
        let bytes = public_key.to_bytes();
        let recovered = PublicKey::from_bytes(&bytes).unwrap();
        assert_eq!(public_key, &recovered);
    }

    #[test]
    fn test_keypair_from_seed_deterministic() {
        let seed = [42u8; 32];
        let keypair1 = KeyPair::from_seed(&seed);
        let keypair2 = KeyPair::from_seed(&seed);
        
        // Les paires de clés générées avec la même graine doivent être identiques
        assert_eq!(keypair1.public_key().to_hex(), keypair2.public_key().to_hex());
        
        let message = b"deterministic test";
        let signature1 = keypair1.sign(message);
        let signature2 = keypair2.sign(message);
        
        // Les signatures doivent être identiques pour le même message et la même clé
        assert_eq!(signature1.to_hex(), signature2.to_hex());
    }

    #[test]
    fn test_private_key_from_bytes() {
        let valid_bytes = [1u8; 32];
        let private_key = PrivateKey::from_bytes(&valid_bytes).unwrap();
        
        // Vérifier que la clé privée peut être convertie en bytes
        let recovered_bytes = private_key.to_bytes();
        assert_eq!(valid_bytes, recovered_bytes);
        
        // Test avec des bytes de mauvaise taille
        let invalid_bytes = [1u8; 31]; // Trop court
        assert!(PrivateKey::from_bytes(&invalid_bytes).is_err());
        
        let invalid_bytes_long = [1u8; 33]; // Trop long
        assert!(PrivateKey::from_bytes(&invalid_bytes_long).is_err());
    }

    #[test]
    fn test_public_key_from_invalid_hex() {
        // Test avec hex invalide
        let invalid_hex = "invalid_hex_string";
        assert!(PublicKey::from_hex(invalid_hex).is_err());
        
        // Test avec hex de mauvaise longueur
        let short_hex = "deadbeef"; // Trop court pour une clé publique
        assert!(PublicKey::from_hex(short_hex).is_err());
    }

    #[test]
    fn test_public_key_from_invalid_bytes() {
        // Test avec des bytes de mauvaise taille
        let invalid_bytes = [0u8; 31]; // Trop court
        assert!(PublicKey::from_bytes(&invalid_bytes).is_err());
        
        let invalid_bytes_long = [0u8; 33]; // Trop long
        assert!(PublicKey::from_bytes(&invalid_bytes_long).is_err());
    }

    #[test]
    fn test_signature_verification_failure() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        let message = b"test message";
        
        // Signer avec une clé et vérifier avec une autre doit échouer
        let signature = keypair1.sign(message);
        assert!(keypair2.verify(message, &signature).is_err());
    }

    #[test]
    fn test_public_key_serde_json() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();
        
        // Test sérialisation JSON
        let serialized = serde_json::to_string(public_key).unwrap();
        
        // Test désérialisation JSON
        let deserialized: PublicKey = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(public_key, &deserialized);
    }

    #[test]
    fn test_private_key_derives_correct_public_key() {
        let seed = [123u8; 32];
        let keypair = KeyPair::from_seed(&seed);
        let private_key = PrivateKey::from_bytes(&seed).unwrap();
        
        // La clé publique dérivée de la clé privée doit correspondre
        let derived_public = private_key.public_key();
        assert_eq!(keypair.public_key().to_hex(), derived_public.to_hex());
    }

    #[test]
    fn test_multiple_signatures_same_message() {
        let keypair = KeyPair::generate();
        let message = b"same message";
        
        // Générer plusieurs signatures du même message
        let signature1 = keypair.sign(message);
        let signature2 = keypair.sign(message);
        
        // Ed25519 est déterministe, les signatures doivent être identiques
        assert_eq!(signature1.to_hex(), signature2.to_hex());
        
        // Les deux signatures doivent être valides
        assert!(keypair.verify(message, &signature1).is_ok());
        assert!(keypair.verify(message, &signature2).is_ok());
    }

    #[test]
    fn test_empty_message_signature() {
        let keypair = KeyPair::generate();
        let empty_message = b"";
        
        // Test signature d'un message vide
        let signature = keypair.sign(empty_message);
        assert!(keypair.verify(empty_message, &signature).is_ok());
    }

    #[test]
    fn test_large_message_signature() {
        let keypair = KeyPair::generate();
        let large_message = vec![42u8; 10000]; // Message de 10KB
        
        // Test signature d'un gros message
        let signature = keypair.sign(&large_message);
        assert!(keypair.verify(&large_message, &signature).is_ok());
    }

    #[test]
    fn test_keypair_uniqueness() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        
        // Deux paires de clés générées aléatoirement doivent être différentes
        assert_ne!(keypair1.public_key().to_hex(), keypair2.public_key().to_hex());
    }

    #[test] 
    fn test_public_key_hex_format() {
        let keypair = KeyPair::generate();
        let hex = keypair.public_key().to_hex();
        
        // Vérifier que le hex a la bonne longueur (32 bytes = 64 chars hex)
        assert_eq!(hex.len(), 64);
        
        // Vérifier que ce sont bien des caractères hexadécimaux
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_cross_verification() {
        let keypair = KeyPair::generate();
        let message = b"cross verification test";
        
        // Signer avec la paire de clés
        let signature_from_keypair = keypair.sign(message);
        
        // Créer une autre paire de clés pour tester la vérification croisée
        let other_keypair = KeyPair::generate();
        let signature_from_other = other_keypair.sign(message);
        
        // Les signatures doivent être différentes
        assert_ne!(signature_from_keypair.to_hex(), signature_from_other.to_hex());
        
        // Vérifier avec la clé publique correcte
        assert!(keypair.public_key().verify(message, &signature_from_keypair).is_ok());
        
        // Vérifier avec la mauvaise clé publique doit échouer
        assert!(keypair.public_key().verify(message, &signature_from_other).is_err());
    }
}

/// Tests pour le module de hachage
mod hash_tests {
    use super::*;

    #[test]
    fn test_hash_creation() {
        let data = b"hello world";
        let hash1 = Hash::new(data);
        let hash2 = Hash::new(data);
        
        // Même données = même hash
        assert_eq!(hash1, hash2);
        
        // Données différentes = hash différent
        let hash3 = Hash::new(b"hello world!");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_serialization() {
        let data = b"test data";
        let hash = Hash::new(data);
        
        // Test conversion hex
        let hex = hash.to_hex();
        let recovered = Hash::from_hex(&hex).unwrap();
        assert_eq!(hash, recovered);
        
        // Test conversion bytes
        let bytes = hash.to_bytes();
        let recovered = Hash::from_bytes(&bytes).unwrap();
        assert_eq!(hash, recovered);
    }

    #[test]
    fn test_zero_hash() {
        let zero = Hash::zero();
        assert!(zero.is_zero());
        
        let data_hash = Hash::new(b"data");
        assert!(!data_hash.is_zero());
    }

    #[test]
    fn test_multi_hash() {
        let elements = vec![b"part1".as_slice(), b"part2".as_slice(), b"part3".as_slice()];
        let multi = Hash::multi_hash(&elements);
        
        let combined = b"part1part2part3";
        let single = Hash::new(combined);
        
        assert_eq!(multi, single);
    }
}

/// Tests pour le module de signature
mod signature_tests {
    use super::*;
    use crypto_lib::Signature;

    #[test]
    fn test_signature_serialization() {
        let keypair = KeyPair::generate();
        let message = b"test message";
        let signature = keypair.sign(message);
        
        // Test conversion hex
        let hex = signature.to_hex();
        let recovered = Signature::from_hex(&hex).unwrap();
        assert_eq!(signature, recovered);
        
        // Test conversion bytes
        let bytes = signature.to_bytes();
        let recovered = Signature::from_bytes(&bytes).unwrap();
        assert_eq!(signature, recovered);
        
        // Vérifier que la signature récupérée fonctionne
        assert!(keypair.verify(message, &recovered).is_ok());
    }
}