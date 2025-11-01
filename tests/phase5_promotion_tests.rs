/// Phase 5 Integration Tests: Automatic Proposal Promotion
/// 
/// Tests the new transaction types and automatic promotion logic
/// when a proposal reaches the configured support threshold.

use common::{
    Blockchain, Transaction, TransactionType, 
    proposal::{Proposal, ProposalStatus},
    Law,
};
use crypto_lib::KeyPair;
use chrono::Utc;
use uuid::Uuid;

/// Helper function to create a properly signed transaction
fn create_signed_transaction(
    tx_type: TransactionType,
    kp: &KeyPair,
    nonce: u64,
    fee: u64,
) -> Transaction {
    let mut tx = Transaction::new(
        tx_type,
        kp.public_key().clone(),
        kp.sign(b"temp"),
        nonce,
        fee,
    );
    
    // Re-sign with proper message
    let msg = format!("TRANSACTION:{}:{}:{}", tx.id, tx.timestamp, tx.data_hash.to_hex());
    tx.signature = kp.sign(msg.as_bytes());
    
    tx
}

#[test]
fn test_phase5_transaction_types() {
    println!("🧪 Testing Phase 5 transaction types");
    
    let kp = KeyPair::generate();
    let proposal_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    let identity_hash = "test_identity_hash".to_string();
    
    // Test IdentityValidated transaction
    let identity_tx = TransactionType::IdentityValidated {
        identity_hash: identity_hash.clone(),
        validator: kp.public_key().clone(),
        timestamp: Utc::now(),
    };
    
    let tx1 = create_signed_transaction(identity_tx, &kp, 1, 0);
    println!("✅ IdentityValidated transaction created: {}", tx1.id);
    
    // Test ProposalCreated transaction
    let proposal_created_tx = TransactionType::ProposalCreated {
        proposal_id,
        author: kp.public_key().clone(),
        title: "Test Proposal".to_string(),
        category: "Test".to_string(),
    };
    
    let tx2 = create_signed_transaction(proposal_created_tx, &kp, 2, 0);
    println!("✅ ProposalCreated transaction created: {}", tx2.id);
    
    // Test SupportAdded transaction
    let support_added_tx = TransactionType::SupportAdded {
        proposal_id,
        supporter: kp.public_key().clone(),
        support_count: 50,
    };
    
    let tx3 = create_signed_transaction(support_added_tx, &kp, 3, 0);
    println!("✅ SupportAdded transaction created: {}", tx3.id);
    
    // Test LawPromoted transaction
    let law_promoted_tx = TransactionType::LawPromoted {
        proposal_id,
        law_id,
        promoted_by: kp.public_key().clone(),
        support_count: 100,
    };
    
    let tx4 = create_signed_transaction(law_promoted_tx, &kp, 4, 0);
    println!("✅ LawPromoted transaction created: {}", tx4.id);
    println!("✅ All Phase 5 transaction types successfully created");
}

#[test]
fn test_proposal_promotion_workflow() {
    println!("🧪 Testing automatic proposal promotion workflow");
    
    let mut blockchain = Blockchain::new();
    let author_kp = KeyPair::generate();
    
    // 1. Create a proposal
    let proposal = Proposal {
        id: Uuid::new_v4(),
        title: "Test Proposal for Promotion".to_string(),
        category: "Infrastructure".to_string(),
        description: "A test proposal to verify automatic promotion".to_string(),
        full_text: "This proposal will be promoted to a law when it reaches the threshold".to_string(),
        estimated_budget: Some(1_000_000),
        implementation_timeline: Some("6 months".to_string()),
        tags: vec!["test".to_string(), "infrastructure".to_string()],
        author_id: Some(author_kp.public_key().to_hex()),
        author_name: Some("Test Author".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(30),
        status: ProposalStatus::CollectingSignatures,
        supporters_count: 0,
    };
    
    // Add proposal to blockchain
    let create_proposal_tx = create_signed_transaction(
        TransactionType::CreateProposal(proposal.clone()),
        &author_kp,
        1,
        0,
    );
    
    blockchain.add_pending_transaction(create_proposal_tx).expect("Failed to add proposal");
    
    // Mine the block to apply the transaction
    let config = common::BlockchainConfig::default();
    blockchain.mine_block(&config).expect("Failed to mine block");
    println!("✅ Proposal created: {}", proposal.id);
    
    // 2. Add support to the proposal (simulate reaching threshold)
    for i in 0..100 {
        let supporter_kp = KeyPair::generate();
        let support_tx = create_signed_transaction(
            TransactionType::SupportProposal {
                proposal_id: proposal.id,
                supporter: supporter_kp.public_key().clone(),
            },
            &supporter_kp,
            i as u64 + 1,
            0,
        );
        
        blockchain.add_pending_transaction(support_tx).expect("Failed to add support");
    }
    
    // Mine the block to apply all support transactions
    blockchain.mine_block(&config).expect("Failed to mine block");
    println!("✅ Added 100 supporters to proposal");
    
    // 3. Verify the proposal state
    if let Some(stored_proposal) = blockchain.proposals.get(&proposal.id) {
        println!("✅ Proposal found in blockchain with {} supporters", stored_proposal.supporters_count);
        assert_eq!(stored_proposal.supporters_count, 100);
    } else {
        panic!("❌ Proposal not found in blockchain");
    }
    
    // 4. Simulate promotion transaction
    let law_id = Uuid::new_v4();
    
    // Add the promoted law first
    let promoted_law = Law::new(
        proposal.title.clone(),
        proposal.full_text.clone(),
        proposal.description.clone(),
        proposal.category.clone(),
        author_kp.public_key().clone(),
        common::law::LawChangeType::Creation,
    );
    
    let create_law_tx = create_signed_transaction(
        TransactionType::CreateLaw(promoted_law.clone()),
        &author_kp,
        102,
        0,
    );
    
    let promotion_tx = create_signed_transaction(
        TransactionType::LawPromoted {
            proposal_id: proposal.id,
            law_id: promoted_law.id,
            promoted_by: author_kp.public_key().clone(),
            support_count: 100,
        },
        &author_kp,
        101,
        0,
    );
    
    blockchain.add_pending_transaction(create_law_tx).expect("Failed to add law");
    blockchain.add_pending_transaction(promotion_tx).expect("Failed to add promotion tx");
    
    // Mine the block to apply the law creation and promotion
    blockchain.mine_block(&config).expect("Failed to mine block");
    println!("✅ Law created and promotion transaction added");
    
    // 5. Verify the law was created
    if let Some(law) = blockchain.laws.get(&promoted_law.id) {
        println!("✅ Law found in blockchain: {}", law.title);
        assert_eq!(law.title, proposal.title);
    } else {
        panic!("❌ Law not found in blockchain");
    }
    
    println!("✅ Automatic promotion workflow completed successfully");
}

#[test]
fn test_support_added_transaction_updates_count() {
    println!("🧪 Testing SupportAdded transaction updates support count");
    
    let mut blockchain = Blockchain::new();
    let kp = KeyPair::generate();
    
    // Create a proposal
    let proposal = Proposal {
        id: Uuid::new_v4(),
        title: "Support Count Test".to_string(),
        category: "Test".to_string(),
        description: "Testing support count updates".to_string(),
        full_text: "Full text".to_string(),
        estimated_budget: None,
        implementation_timeline: None,
        tags: vec![],
        author_id: Some(kp.public_key().to_hex()),
        author_name: Some("Test".to_string()),
        created_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::days(30),
        status: ProposalStatus::CollectingSignatures,
        supporters_count: 0,
    };
    
    let create_tx = create_signed_transaction(
        TransactionType::CreateProposal(proposal.clone()),
        &kp,
        1,
        0,
    );
    
    blockchain.add_pending_transaction(create_tx).expect("Failed to create proposal");
    
    // Mine to apply the proposal creation
    let config = common::BlockchainConfig::default();
    blockchain.mine_block(&config).expect("Failed to mine block");
    
    // Add a SupportAdded transaction with count 42
    let support_added_tx = create_signed_transaction(
        TransactionType::SupportAdded {
            proposal_id: proposal.id,
            supporter: kp.public_key().clone(),
            support_count: 42,
        },
        &kp,
        2,
        0,
    );
    
    blockchain.add_pending_transaction(support_added_tx).expect("Failed to add support");
    
    // Mine to apply the support added transaction
    blockchain.mine_block(&config).expect("Failed to mine block");
    
    // Verify the count was updated
    if let Some(stored_proposal) = blockchain.proposals.get(&proposal.id) {
        assert_eq!(stored_proposal.supporters_count, 42, "Support count should be updated to 42");
        println!("✅ Support count correctly updated to {}", stored_proposal.supporters_count);
    } else {
        panic!("❌ Proposal not found");
    }
}

#[test]
fn test_identity_validated_transaction() {
    println!("🧪 Testing IdentityValidated transaction processing");
    
    let mut blockchain = Blockchain::new();
    let validator_kp = KeyPair::generate();
    let identity_hash = "test_hash_12345".to_string();
    
    let identity_tx = create_signed_transaction(
        TransactionType::IdentityValidated {
            identity_hash: identity_hash.clone(),
            validator: validator_kp.public_key().clone(),
            timestamp: Utc::now(),
        },
        &validator_kp,
        1,
        0,
    );
    
    // This transaction should be accepted as it's primarily for audit trail
    blockchain.add_pending_transaction(identity_tx).expect("Failed to add identity validation");
    
    println!("✅ IdentityValidated transaction successfully processed");
}

#[test]
fn test_law_promoted_requires_existing_proposal_and_law() {
    println!("🧪 Testing LawPromoted transaction validation");
    
    let mut blockchain = Blockchain::new();
    let kp = KeyPair::generate();
    let proposal_id = Uuid::new_v4();
    let law_id = Uuid::new_v4();
    
    // Try to add a LawPromoted transaction without the proposal or law existing
    let promotion_tx = create_signed_transaction(
        TransactionType::LawPromoted {
            proposal_id,
            law_id,
            promoted_by: kp.public_key().clone(),
            support_count: 100,
        },
        &kp,
        1,
        0,
    );
    
    // This should be accepted in mempool but will fail when mining
    blockchain.add_pending_transaction(promotion_tx).expect("Should accept to mempool");
    
    // Try to mine - this should fail validation
    let config = common::BlockchainConfig::default();
    let result = blockchain.mine_block(&config);
    assert!(result.is_err(), "Should fail mining when proposal doesn't exist");
    println!("✅ Correctly rejected LawPromoted for non-existent proposal during mining");
}
