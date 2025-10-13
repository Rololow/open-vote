//! ZKP Prelude: traits and minimal types to prepare Phase 3 (anonymous proofs).
//!
//! This module intentionally avoids bringing any heavy dependencies.
//! It defines a small trait for an identity commitment accumulator
//! that future ZK circuits can bind to.

use crate::errors::Result;
use serde::{Serialize, Deserialize};

/// A 32-byte identity commitment hash (SHA-256 hex elsewhere in the system).
pub type Commitment = [u8; 32];

/// Trait for append-only accumulators (e.g., Merkle) that produce a succinct root.
pub trait IdentityAccumulator {
    /// Append a new commitment leaf.
    fn append(&mut self, leaf: Commitment) -> Result<()>;
    /// Return the current root hash (32 bytes).
    fn root(&self) -> Commitment;
    /// Return number of leaves.
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}

/// A minimal in-memory Merkle-like accumulator placeholder.
/// This is NOT optimized and uses a simple pairwise hash fold.
#[derive(Debug, Default, Clone)]
pub struct MerkleAccumulator {
    leaves: Vec<Commitment>,
}

impl MerkleAccumulator {
    pub fn new() -> Self { Self { leaves: Vec::new() } }
}

impl IdentityAccumulator for MerkleAccumulator {
    fn append(&mut self, leaf: Commitment) -> Result<()> {
        self.leaves.push(leaf);
        Ok(())
    }

    fn root(&self) -> Commitment {
        use sha2::{Digest, Sha256};
        if self.leaves.is_empty() { return [0u8; 32]; }
        // Work buffer: start from leaves
        let mut level = self.leaves.clone();
        while level.len() > 1 {
            let mut next = Vec::with_capacity((level.len()+1)/2);
            let mut i = 0;
            while i < level.len() {
                let a = level[i];
                let b = if i+1 < level.len() { level[i+1] } else { level[i] }; // duplicate last if odd
                let mut hasher = Sha256::new();
                hasher.update(&a);
                hasher.update(&b);
                let out = hasher.finalize();
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&out);
                next.push(arr);
                i += 2;
            }
            level = next;
        }
        level[0]
    }

    fn len(&self) -> usize { self.leaves.len() }
}

/// Sibling path proof for a Merkle leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// Sibling hashes from leaf level upward; at each level, the sibling of the current hash.
    pub siblings: Vec<Commitment>,
    /// Zero-based leaf index in the original leaves list.
    pub leaf_index: usize,
}

/// Generate a Merkle proof for a given leaf index.
pub fn merkle_proof_for(leaves: &[Commitment], index: usize) -> Option<MerkleProof> {
    if leaves.is_empty() || index >= leaves.len() { return None; }
    let mut siblings = Vec::new();
    let mut idx = index;
    let mut level: Vec<Commitment> = leaves.to_vec();
    while level.len() > 1 {
        let is_right = idx % 2 == 1;
        let sib_idx = if is_right { idx - 1 } else { idx + 1 };
        let sibling = if sib_idx < level.len() { level[sib_idx] } else { level[idx] };
        siblings.push(sibling);
        // move up
        idx /= 2;
        // compute next level
        use sha2::{Digest, Sha256};
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        let mut i = 0;
        while i < level.len() {
            let a = level[i];
            let b = if i + 1 < level.len() { level[i + 1] } else { level[i] };
            let mut hasher = Sha256::new();
            hasher.update(&a);
            hasher.update(&b);
            let out = hasher.finalize();
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&out);
            next.push(arr);
            i += 2;
        }
        level = next;
    }
    Some(MerkleProof { siblings, leaf_index: index })
}

/// Verify that a leaf with its proof yields the expected Merkle root.
pub fn verify_merkle_proof(leaf: &Commitment, proof: &MerkleProof, expected_root: &Commitment) -> bool {
    use sha2::{Digest, Sha256};
    let mut hash = *leaf;
    let mut idx = proof.leaf_index;
    for sib in &proof.siblings {
        let (left, right) = if idx % 2 == 0 { (&hash, sib) } else { (sib, &hash) };
        let mut hasher = Sha256::new();
        hasher.update(left);
        hasher.update(right);
        let out = hasher.finalize();
        hash.copy_from_slice(&out);
        idx /= 2;
    }
    &hash == expected_root
}

/// ZKP scheme enumeration used in proof envelopes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProofScheme {
    Groth16,
    Halo2,
}

/// Versioned proof envelope for anonymous actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEnvelope {
    pub scheme: ProofScheme,
    pub vk_version: u32,
    /// Merkle root in hex (lowercase) this proof binds to
    pub root_hex: String,
    /// Action scope (e.g., "vote:LAW_UUID" or "support:PROPOSAL_UUID")
    pub scope: String,
    /// Nullifier in hex (domain-separated inside the circuit)
    pub nullifier_hex: String,
    /// Serialized proof bytes (encoding scheme depends on `scheme`)
    pub proof: Vec<u8>,
    /// Optional public inputs (hex/strings) for transparency/debug
    #[serde(default)]
    pub public_inputs: Vec<String>,
}

/// Generic anonymous action payload carried by transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymousActionPayload {
    pub proof_envelope: ProofEnvelope,
    /// Optional extra fields that the application layer may need.
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
}

/// Compute a deterministic hex-encoded 32-byte nullifier for a given action scope and user secret.
/// This is a placeholder suitable for mock/testing; the real circuit should bind the same formula.
/// secret_bytes: a stable 32-byte per-identity secret (e.g., hash of private key)
pub fn compute_scoped_nullifier_hex(scope: &str, secret_bytes: &[u8; 32]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"nullifier:");
    h.update(scope.as_bytes());
    h.update(b":");
    h.update(secret_bytes);
    let out = h.finalize();
    hex::encode(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn h(b: &[u8]) -> Commitment {
        let mut out = [0u8; 32];
        let dig = Sha256::digest(b);
        out.copy_from_slice(&dig);
        out
    }

    #[test]
    fn empty_root_is_zero() {
        let acc = MerkleAccumulator::new();
        assert_eq!(acc.root(), [0u8; 32]);
    }

    #[test]
    fn small_tree_roots() {
        let mut acc = MerkleAccumulator::new();
        acc.append(h(b"a")).unwrap();
        let r1 = acc.root();
        acc.append(h(b"b")).unwrap();
        let r2 = acc.root();
        assert_ne!(r1, [0u8; 32]);
        assert_ne!(r2, r1);
        // add third (odd) and ensure deterministic
        acc.append(h(b"c")).unwrap();
        let r3 = acc.root();
        // stability: appending changes root
        assert_ne!(r3, r2);
    }

    #[test]
    fn merkle_proof_roundtrip_even() {
        let leaves = vec![h(b"a"), h(b"b"), h(b"c"), h(b"d")];
        let mut acc = MerkleAccumulator::new();
        for l in &leaves { acc.append(*l).unwrap(); }
        let root = acc.root();
        for i in 0..leaves.len() {
            let p = merkle_proof_for(&leaves, i).expect("proof");
            assert!(verify_merkle_proof(&leaves[i], &p, &root));
        }
    }

    #[test]
    fn merkle_proof_roundtrip_odd() {
        let leaves = vec![h(b"a"), h(b"b"), h(b"c")];
        let mut acc = MerkleAccumulator::new();
        for l in &leaves { acc.append(*l).unwrap(); }
        let root = acc.root();
        for i in 0..leaves.len() {
            let p = merkle_proof_for(&leaves, i).expect("proof");
            assert!(verify_merkle_proof(&leaves[i], &p, &root));
        }
    }
}
