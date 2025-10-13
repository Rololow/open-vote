//! Shared API types for the e-government blockchain system
//!
//! This module contains common types used across different components
//! of the system, particularly for attestation, signatures, and API
//! communication between wallet, government server, and blockchain.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a government attestation for citizen identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationPayload {
    /// Unique user public key (Ed25519)
    pub user_pubkey: String,
    /// SHA-256 hash of the identity data
    pub identity_hash: String,
    /// Issuance timestamp (Unix timestamp)
    pub issuance_ts: u64,
    /// Expiry timestamp (Unix timestamp)
    pub expiry_ts: u64,
    /// Version of the attestation format
    pub version: u32,
    /// Unique nonce to prevent replay attacks
    pub nonce: String,
}

/// A signed attestation from the government server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAttestation {
    /// The attestation payload
    pub payload: AttestationPayload,
    /// Base64-encoded signature from government private key
    pub signature: String,
}

/// Request to create a new proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedProposal {
    /// Unique proposal ID
    pub id: Uuid,
    /// Proposer's public key
    pub proposer_pubkey: String,
    /// Proposal title
    pub title: String,
    /// Proposal description/body
    pub description: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Base64-encoded signature
    pub signature: String,
}

/// Request to support a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedSupport {
    /// Proposal ID being supported
    pub proposal_id: Uuid,
    /// Supporter's public key
    pub supporter_pubkey: String,
    /// Support timestamp
    pub timestamp: u64,
    /// Base64-encoded signature
    pub signature: String,
}

/// Vote submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedVote {
    /// Law ID being voted on
    pub law_id: Uuid,
    /// Voter's public key
    pub voter_pubkey: String,
    /// Vote choice
    pub choice: VoteChoice,
    /// Vote timestamp
    pub timestamp: u64,
    /// Base64-encoded signature
    pub signature: String,
}

/// Vote choices available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

/// Identity commitment for anonymous actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCommitment {
    /// SHA-256 hash of the commitment
    pub hash: String,
    /// Issuer DID
    pub issuer_did: String,
    /// Subject DID
    pub subject_did: String,
    /// Issuance timestamp
    pub issued_at: u64,
    /// Expiry timestamp
    pub expires_at: Option<u64>,
    /// Commitment status
    pub status: CommitmentStatus,
}

/// Status of an identity commitment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitmentStatus {
    Active,
    Revoked,
    Expired,
}

/// Merkle proof for identity membership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Root hash of the Merkle tree
    pub root: String,
    /// Leaf index in the tree
    pub leaf_index: usize,
    /// Sibling hashes for proof verification
    pub siblings: Vec<String>,
}

/// ZKP proof envelope for anonymous transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEnvelope {
    /// The ZKP proof data (serialized)
    pub proof: Vec<u8>,
    /// Public inputs to the proof
    pub public_inputs: Vec<String>,
    /// Proof system used (e.g., "groth16")
    pub proof_system: String,
    /// Circuit identifier
    pub circuit_id: String,
}

/// Anonymous support transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousSupport {
    /// Proposal ID being supported
    pub proposal_id: Uuid,
    /// ZKP proof of membership and uniqueness
    pub proof: ProofEnvelope,
    /// Scope for nullifier (prevents double-support)
    pub scope: String,
}

/// Anonymous vote transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousVote {
    /// Law ID being voted on
    pub law_id: Uuid,
    /// Vote choice
    pub choice: VoteChoice,
    /// ZKP proof of membership and uniqueness
    pub proof: ProofEnvelope,
    /// Scope for nullifier (prevents double-voting)
    pub scope: String,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success flag
    pub success: bool,
    /// Response data
    pub data: Option<T>,
    /// Error message (if success is false)
    pub error: Option<String>,
    /// Timestamp of response
    pub timestamp: u64,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (0-based)
    pub page: Option<usize>,
    /// Items per page
    pub limit: Option<usize>,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Items for this page
    pub items: Vec<T>,
    /// Total number of items
    pub total: usize,
    /// Current page number
    pub page: usize,
    /// Items per page
    pub limit: usize,
    /// Total number of pages
    pub total_pages: usize,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Create an error response
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}