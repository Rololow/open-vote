#![cfg(feature = "zkp_groth16")]

use std::fs;
use std::io::Write;

use ark_bn254::{Bn254, Fr};
use ark_ff::Field;
use ark_groth16::{prepare_verifying_key, Groth16, VerifyingKey};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, LinearCombination};
use ark_snark::{SNARK, CircuitSpecificSetupSNARK};
use rand::thread_rng;

// Import the verifier helpers from the crate under test
use blockchain_server::zkp_verifier::{verify_groth16_bn254, vk_path_for_version, load_poseidon_params};

// A tiny circuit that enforces: a + b = c where
// - public inputs are [c, a, b] (3 public inputs)
// This lets us simulate mapping [root, scope_hash, nullifier] into 3 Frs.
#[derive(Clone, Default)]
struct AddCircuit<F: Field> {
    pub a: Option<F>, // witness
    pub b: Option<F>, // witness
    pub c: Option<F>, // public
}

impl<F: Field> ConstraintSynthesizer<F> for AddCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // Allocate witness variables
    let a_val = self.a.ok_or(SynthesisError::AssignmentMissing)?;
    let b_val = self.b.ok_or(SynthesisError::AssignmentMissing)?;
    let c_val = self.c.ok_or(SynthesisError::AssignmentMissing)?;

    let a_var = cs.new_witness_variable(|| Ok(a_val))?;
    let b_var = cs.new_witness_variable(|| Ok(b_val))?;
    let c_var = cs.new_input_variable(|| Ok(c_val))?;

        // Enforce a + b = c
        // Build linear combination: a + b - c = 0
        let mut lc = LinearCombination::<F>::zero();
        lc = lc + (F::one(), a_var);
        lc = lc + (F::one(), b_var);
        lc = lc + (-F::one(), c_var);
        cs.enforce_constraint(lc, LinearCombination::zero(), LinearCombination::zero())?;
        Ok(())
    }
}

#[test]
fn groth16_real_vk_and_proof_roundtrip() {
    // Setup proving/verifying keys for the circuit once (arkworks 0.4 SNARK API)
    let mut rng = thread_rng();
    // Provide concrete assignments (0 + 0 = 0) for setup to avoid AssignmentMissing
    let setup_circuit = AddCircuit::<Fr> { a: Some(Fr::from(0u64)), b: Some(Fr::from(0u64)), c: Some(Fr::from(0u64)) };
    let (pk, vk): (_, VerifyingKey<Bn254>) = Groth16::<Bn254>::setup(setup_circuit, &mut rng)
        .expect("setup");
    let pvk = prepare_verifying_key(&vk);

    // Create a temp data dir and write the verifying key in compressed form to the expected path
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_str().unwrap();
    let vk_path = vk_path_for_version(data_dir, 123);
    fs::create_dir_all(vk_path.parent().unwrap()).unwrap();
    let mut vk_bytes = Vec::new();
    ark_serialize::CanonicalSerialize::serialize_compressed(&vk, &mut vk_bytes).unwrap();
    let mut f = fs::File::create(&vk_path).unwrap();
    f.write_all(&vk_bytes).unwrap();
    drop(f);

    // Choose small values for a, b such that c = a + b
    let a = Fr::from(5u64);
    let b = Fr::from(7u64);
    let c = a + b; // 12

    // Synthesize a proof for this instance
    let circuit = AddCircuit { a: Some(a), b: Some(b), c: Some(c) };
    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove");

    // Serialize proof to bytes (compressed)
    let mut proof_bytes = Vec::new();
    ark_serialize::CanonicalSerialize::serialize_compressed(&proof, &mut proof_bytes).unwrap();

    // Verify directly with arkworks to sanity check
    let ok_direct = Groth16::<Bn254>::verify_proof(&pvk, &proof, &[c]).expect("verify_direct");
    assert!(ok_direct, "arkworks direct verification failed");

    // Now call our verifier function using the same public inputs order [c]
    // Load poseidon params (tests may not have a real params file; fall back to a default)
    let poseidon_params = match load_poseidon_params(data_dir) {
        Ok(p) => std::sync::Arc::new(p),
        Err(_) => std::sync::Arc::new(ark_crypto_primitives::sponge::poseidon::PoseidonConfig::new(8, 57, 5, vec![], vec![], 2, 1)),
    };
    let ok_indirect = verify_groth16_bn254(data_dir, 123, &proof_bytes, &[c], &poseidon_params).expect("verifier");
    assert!(ok_indirect, "indirect verification failed");
}
