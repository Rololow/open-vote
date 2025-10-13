use common::identity::zkp_prelude::{Commitment, MerkleAccumulator, merkle_proof_for, verify_merkle_proof, IdentityAccumulator};

fn parse_hex32(s: &str) -> Option<Commitment> {
    let t = s.trim();
    if t.len() != 64 { return None; }
    let mut out = [0u8; 32];
    hex::decode_to_slice(t, &mut out).ok()?;
    Some(out)
}

#[test]
fn merkle_proof_hex_parsing_and_verify() {
    // Three known SHA-256 digests for strings "a", "b", "c"
    let lines = [
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb",
        "3a7bd3e2360a3d80a59c8c46f1a74bafa3015dbb0fcd75c0f9b5f3a5d66a1f9d",
        "2e7d2c03a9507ae265ecf5b5356885a53393a2029d241394997265a1a25aefc6",
    ];

    let mut leaves: Vec<Commitment> = Vec::new();
    for l in &lines {
        leaves.push(parse_hex32(l).expect("hex32"));
    }

    // Build accumulator and compute root
    let mut acc = MerkleAccumulator::new();
    for c in &leaves { IdentityAccumulator::append(&mut acc, *c).unwrap(); }
    let root = IdentityAccumulator::root(&acc);

    // Build and verify a proof for each index
    for i in 0..leaves.len() {
        let proof = merkle_proof_for(&leaves, i).expect("proof");
        assert!(verify_merkle_proof(&leaves[i], &proof, &root));
    }
}
