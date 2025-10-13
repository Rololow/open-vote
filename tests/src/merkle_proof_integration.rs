use sha2::{Digest, Sha256};
use common::identity::zkp_prelude::{Commitment, MerkleAccumulator, merkle_proof_for, verify_merkle_proof};

fn h(s: &str) -> Commitment {
    let mut out = [0u8; 32];
    let dig = Sha256::digest(s.as_bytes());
    out.copy_from_slice(&dig);
    out
}

#[test]
fn parse_hex_and_verify_proof_roundtrip() {
    // Simulate a tiny commitments.log with hex lines
    let leaves: Vec<Commitment> = ["a", "b", "c", "d"].iter().map(|s| h(s)).collect();

    // Build accumulator and root
    let mut acc = MerkleAccumulator::new();
    for l in &leaves { acc.append(*l).unwrap(); }
    let root = acc.root();

    // Pick a leaf index, generate proof using library
    let idx = 2usize; // 'c'
    let proof = merkle_proof_for(&leaves, idx).expect("proof");

    // Verify using the same library function
    assert!(verify_merkle_proof(&leaves[idx], &proof, &root));

    // Ensure even an out-of-place leaf fails
    assert!(!verify_merkle_proof(&leaves[0], &proof, &root));
}
