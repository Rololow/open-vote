use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use common::{Blockchain, BlockchainConfig, Transaction, TransactionType, Account, Law, Vote, VoteType, LawChangeType};
use crypto_lib::{KeyPair, Hash};

/// Tests d'intégration complets du système e-gouvernement
#[tokio::test]
async fn test_complete_e_government_workflow() {
    println!("🧪 Test du workflow complet e-gouvernement");
    
    // 1. Initialiser la blockchain
    let mut blockchain = Blockchain::new();
    let config = BlockchainConfig::default();
    
    // 2. Créer des utilisateurs
    let citizen1_keypair = KeyPair::generate();
    let citizen2_keypair = KeyPair::generate();
    let lawmaker_keypair = KeyPair::generate();
    
    println!("✅ Utilisateurs créés");
    
    // 3. Créer des comptes
    let citizen1_account = Account::new(citizen1_keypair.public_key().clone());
    let mut citizen2_account = Account::new(citizen2_keypair.public_key().clone());
    let mut lawmaker_account = Account::new(lawmaker_keypair.public_key().clone());
    
    // Donner de la réputation au législateur
    lawmaker_account.reputation = 500;
    lawmaker_account.metadata.verified = true;
    lawmaker_account.metadata.specializations.push("Droit constitutionnel".to_string());
    
    // Ajouter les comptes à la blockchain via des transactions
    let create_citizen1_tx = Transaction::new(
        TransactionType::CreateAccount(citizen1_account.clone()),
        citizen1_keypair.public_key().clone(),
        citizen1_keypair.sign(b"create_account"),
        1,
        10,
    );
    
    let create_citizen2_tx = Transaction::new(
        TransactionType::CreateAccount(citizen2_account.clone()),
        citizen2_keypair.public_key().clone(),
        citizen2_keypair.sign(b"create_account"),
        1,
        10,
    );
    
    let create_lawmaker_tx = Transaction::new(
        TransactionType::CreateAccount(lawmaker_account.clone()),
        lawmaker_keypair.public_key().clone(),
        lawmaker_keypair.sign(b"create_account"),
        1,
        10,
    );
    
    blockchain.add_pending_transaction(create_citizen1_tx).unwrap();
    blockchain.add_pending_transaction(create_citizen2_tx).unwrap();
    blockchain.add_pending_transaction(create_lawmaker_tx).unwrap();
    
    // Miner le premier bloc avec les comptes
    let block1 = blockchain.mine_block(&config).unwrap();
    println!("✅ Bloc 1 miné avec {} comptes", block1.transactions.len());
    
    // 4. Créer une proposition de loi
    let law = Law::new(
        "Loi sur la protection des données personnelles".to_string(),
        "Article 1: Toute donnée personnelle doit être protégée...".to_string(),
        "Protection renforcée des données citoyens".to_string(),
        "Protection des données".to_string(),
        lawmaker_keypair.public_key().clone(),
        LawChangeType::Creation,
    );
    
    let create_law_tx = Transaction::new(
        TransactionType::CreateLaw(law.clone()),
        lawmaker_keypair.public_key().clone(),
        lawmaker_keypair.sign(b"create_law"),
        2,
        50,
    );
    
    blockchain.add_pending_transaction(create_law_tx).unwrap();
    
    // Miner le bloc avec la loi
    let block2 = blockchain.mine_block(&config).unwrap();
    println!("✅ Bloc 2 miné avec la nouvelle loi");
    
    // 5. Démarrer le vote sur la loi
    let mut law_for_voting = law.clone();
    law_for_voting.start_voting(168).unwrap(); // Vote ouvert pour 1 semaine
    
    let update_law_tx = Transaction::new(
        TransactionType::UpdateLaw(law.id, law_for_voting.clone()),
        lawmaker_keypair.public_key().clone(),
        lawmaker_keypair.sign(b"start_voting"),
        3,
        25,
    );
    
    blockchain.add_pending_transaction(update_law_tx).unwrap();
    
    // 6. Les citoyens votent
    let vote1 = Vote::new(
        law.id,
        citizen1_keypair.public_key().clone(),
        VoteType::For,
        citizen1_account.vote_weight_for_topic("Protection des données"),
        Some("Je soutiens cette proposition importante".to_string()),
        citizen1_keypair.sign(format!("VOTE:{}:For", law.id).as_bytes()),
    );
    
    let vote2 = Vote::new(
        law.id,
        citizen2_keypair.public_key().clone(),
        VoteType::Against,
        citizen2_account.vote_weight_for_topic("Protection des données"),
        Some("Trop restrictif pour les entreprises".to_string()),
        citizen2_keypair.sign(format!("VOTE:{}:Against", law.id).as_bytes()),
    );
    
    let vote3 = Vote::new(
        law.id,
        lawmaker_keypair.public_key().clone(),
        VoteType::For,
        lawmaker_account.vote_weight_for_topic("Protection des données"),
        Some("Expertise juridique: nécessaire".to_string()),
        lawmaker_keypair.sign(format!("VOTE:{}:For", law.id).as_bytes()),
    );
    
    let submit_vote1_tx = Transaction::new(
        TransactionType::SubmitVote(vote1.clone()),
        citizen1_keypair.public_key().clone(),
        citizen1_keypair.sign(b"submit_vote"),
        4,
        15,
    );
    
    let submit_vote2_tx = Transaction::new(
        TransactionType::SubmitVote(vote2.clone()),
        citizen2_keypair.public_key().clone(),
        citizen2_keypair.sign(b"submit_vote"),
        4,
        15,
    );
    
    let submit_vote3_tx = Transaction::new(
        TransactionType::SubmitVote(vote3.clone()),
        lawmaker_keypair.public_key().clone(),
        lawmaker_keypair.sign(b"submit_vote"),
        5,
        15,
    );
    
    blockchain.add_pending_transaction(submit_vote1_tx).unwrap();
    blockchain.add_pending_transaction(submit_vote2_tx).unwrap();
    blockchain.add_pending_transaction(submit_vote3_tx).unwrap();
    
    // Miner le bloc avec les votes
    let block3 = blockchain.mine_block(&config).unwrap();
    println!("✅ Bloc 3 miné avec {} votes", block3.transactions.len());
    
    // 7. Calculer les résultats du vote
    let vote_results = blockchain.calculate_vote_results(&law.id).unwrap();
    
    println!("\n📊 Résultats du vote:");
    println!("   Total votes: {}", vote_results.total_votes);
    println!("   Pour: {} (poids: {:.2})", vote_results.votes_for, vote_results.weighted_for);
    println!("   Contre: {} (poids: {:.2})", vote_results.votes_against, vote_results.weighted_against);
    println!("   Taux d'approbation: {:.1}%", vote_results.weighted_approval_rate * 100.0);
    
    // 8. Vérifications
    assert_eq!(blockchain.height(), 3, "La blockchain devrait avoir 3 blocs");
    assert_eq!(blockchain.accounts.len(), 3, "Il devrait y avoir 3 comptes");
    assert_eq!(blockchain.laws.len(), 1, "Il devrait y avoir 1 loi");
    
    let votes_for_law = blockchain.get_votes_for_law(&law.id);
    assert_eq!(votes_for_law.len(), 3, "Il devrait y avoir 3 votes");
    
    // La loi devrait être approuvée (le vote expert pèse plus lourd)
    assert!(vote_results.weighted_approval_rate > 0.5, "La loi devrait être approuvée");
    
    // 9. Test de la validation de la chaîne
    blockchain.verify_chain().unwrap();
    println!("✅ Validation complète de la blockchain réussie");
    
    println!("\n🎉 Test complet du workflow e-gouvernement réussi !");
}

#[tokio::test]
async fn test_blockchain_integrity() {
    println!("🧪 Test d'intégrité de la blockchain");
    
    let mut blockchain = Blockchain::new();
    let config = BlockchainConfig::default();
    
    // Créer quelques transactions factices
    let keypair = KeyPair::generate();
    let account = Account::new(keypair.public_key().clone());
    
    for i in 0..5 {
        let tx = Transaction::new(
            TransactionType::CreateAccount(account.clone()),
            keypair.public_key().clone(),
            keypair.sign(format!("tx_{}", i).as_bytes()),
            i + 1,
            10,
        );
        
        blockchain.add_pending_transaction(tx).unwrap();
        
        if i % 2 == 1 {
            blockchain.mine_block(&config).unwrap();
        }
    }
    
    // Vérifier l'intégrité
    blockchain.verify_chain().unwrap();
    
    // Vérifier les hashes des blocs
    for i in 1..blockchain.blocks.len() {
        let current_block = &blockchain.blocks[i];
        let previous_block = &blockchain.blocks[i - 1];
        
        assert_eq!(
            current_block.header.previous_hash,
            previous_block.hash,
            "Le hash du bloc précédent ne correspond pas"
        );
        
        assert_eq!(
            current_block.header.block_number,
            previous_block.header.block_number + 1,
            "Le numéro de bloc n'est pas séquentiel"
        );
    }
    
    println!("✅ Test d'intégrité réussi pour {} blocs", blockchain.blocks.len());
}

#[tokio::test]
async fn test_cryptographic_security() {
    println!("🧪 Test de sécurité cryptographique");
    
    // Test des clés et signatures
    let keypair1 = KeyPair::generate();
    let keypair2 = KeyPair::generate();
    
    let message = b"message secret";
    let signature1 = keypair1.sign(message);
    
    // La signature doit être valide avec la bonne clé
    assert!(keypair1.verify(message, &signature1).is_ok());
    
    // La signature ne doit pas être valide avec une autre clé
    assert!(keypair2.verify(message, &signature1).is_err());
    
    // La signature ne doit pas être valide avec un autre message
    assert!(keypair1.verify(b"autre message", &signature1).is_err());
    
    // Test des hashes
    let data1 = b"données importantes";
    let data2 = b"autres données";
    
    let hash1 = Hash::new(data1);
    let hash2 = Hash::new(data1); // Même données
    let hash3 = Hash::new(data2); // Données différentes
    
    assert_eq!(hash1, hash2, "Les hashes de mêmes données doivent être identiques");
    assert_ne!(hash1, hash3, "Les hashes de données différentes doivent être différents");
    
    println!("✅ Tests cryptographiques réussis");
}

#[tokio::test]
async fn test_vote_weight_calculation() {
    println!("🧪 Test de calcul des poids de vote");
    
    // Créer des comptes avec différentes réputations
    let mut basic_account = Account::new(KeyPair::generate().public_key().clone());
    basic_account.reputation = 10;
    
    let mut experienced_account = Account::new(KeyPair::generate().public_key().clone());
    experienced_account.reputation = 1000;
    experienced_account.metadata.verified = true;
    
    let mut expert_account = Account::new(KeyPair::generate().public_key().clone());
    expert_account.reputation = 5000;
    expert_account.metadata.verified = true;
    expert_account.metadata.specializations.push("Environnement".to_string());
    
    // Test des poids pour différents sujets
    let basic_weight = basic_account.vote_weight_for_topic("Environnement");
    let experienced_weight = experienced_account.vote_weight_for_topic("Environnement");
    let expert_weight = expert_account.vote_weight_for_topic("Environnement");
    
    println!("   Poids basique: {:.2}", basic_weight);
    println!("   Poids expérimenté: {:.2}", experienced_weight);
    println!("   Poids expert: {:.2}", expert_weight);
    
    // L'expert devrait avoir le poids le plus élevé
    assert!(expert_weight > experienced_weight);
    assert!(experienced_weight > basic_weight);
    
    // Test sans spécialisation
    let expert_weight_other = expert_account.vote_weight_for_topic("Économie");
    assert!(expert_weight > expert_weight_other, "Le bonus de spécialisation devrait s'appliquer");
    
    println!("✅ Calculs de poids de vote corrects");
}
