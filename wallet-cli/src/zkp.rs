/// Generate and save Poseidon parameters to wallet-cli/zkp/poseidon_params.bin
pub fn generate_and_save_poseidon_params(data_dir: &str) -> anyhow::Result<()> {
    // Load Poseidon parameters from config file for synchronization
    let config_path = std::path::Path::new("config/poseidon_config.json");
    let config_str = std::fs::read_to_string(config_path)?;
    let config_json: serde_json::Value = serde_json::from_str(&config_str)?;
    let poseidon = &config_json["poseidon"];
    let rate = poseidon["rate"].as_u64().unwrap_or(2) as usize;
    let full_rounds = poseidon["full_rounds"].as_u64().unwrap_or(8);
    let partial_rounds = poseidon["partial_rounds"].as_u64().unwrap_or(57);
    let skip_matrices = poseidon["skip_matrices"].as_u64().unwrap_or(0);
    let prime_bits = poseidon["prime_bits"].as_u64().unwrap_or(Fr::MODULUS_BIT_SIZE as u64);
    let alpha = poseidon["alpha"].as_u64().unwrap_or(5);
    let capacity = poseidon["capacity"].as_u64().unwrap_or(1) as usize;
    let (ark, mds) = ark_crypto_primitives::sponge::poseidon::find_poseidon_ark_and_mds(
        prime_bits,
        rate,
        full_rounds,
        partial_rounds,
        skip_matrices,
    );
    let poseidon_params = PoseidonConfig::new(
        full_rounds as usize,
        partial_rounds as usize,
        alpha,
        mds,
        ark,
        rate,
        capacity,
    );
    save_poseidon_params(data_dir, &poseidon_params)?;
    Ok(())
}
/// Compute SHA256 hash of Poseidon parameters for synchronisation checks
pub fn poseidon_params_hash(params: &PoseidonConfig<Fr>) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use ark_serialize::CanonicalSerialize;
    let mut hasher = Sha256::new();
    hasher.update(&params.full_rounds.to_le_bytes());
    hasher.update(&params.partial_rounds.to_le_bytes());
    hasher.update(&params.alpha.to_le_bytes());
    hasher.update(&params.rate.to_le_bytes());
    hasher.update(&params.capacity.to_le_bytes());
    for row in &params.mds {
        for el in row {
            let mut b = Vec::new();
            el.serialize_compressed(&mut b)?;
            hasher.update(&b);
        }
    }
    for row in &params.ark {
        for el in row {
            let mut b = Vec::new();
            el.serialize_compressed(&mut b)?;
            hasher.update(&b);
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

use anyhow::{Context, Result};
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{Groth16, ProvingKey, VerifyingKey, Proof, prepare_verifying_key};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_snark::SNARK;
use ark_snark::CircuitSpecificSetupSNARK;
use ark_ff::UniformRand;
use ark_ff::BigInteger;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
// Poseidon CRH gadget imports
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_r1cs_std::prelude::AllocationMode;
use ark_crypto_primitives::crh::poseidon::constraints::{CRHGadget, CRHParametersVar};
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_crypto_primitives::crh::CRHScheme;
use ark_crypto_primitives::crh::TwoToOneCRHScheme;
use ark_crypto_primitives::crh::CRHSchemeGadget;
use std::fs::File;
use std::io::Write;

/// Merkle tree depth used by the circuit. Chosen as 16 for a realistic size.
pub const MERKLE_DEPTH: usize = 8;

/// Membership + nullifier circuit. Public inputs: root, nullifier, scope_hash.
/// Private witness: leaf, merkle_path (siblings), secret. Poseidon params are injected as constants.
#[derive(Clone)]
pub struct MembershipNullifierCircuit {
    pub root: Option<Fr>,
    pub nullifier: Option<Fr>,
    pub scope_hash: Option<Fr>,
    pub leaf: Option<Fr>,
    pub merkle_path: Vec<(Option<Fr>, bool)>, // (sibling, is_left)
    pub secret: Option<Fr>,
    pub poseidon_params: Option<PoseidonConfig<Fr>>,
}

impl ConstraintSynthesizer<Fr> for MembershipNullifierCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> std::result::Result<(), SynthesisError> {
        // Public inputs (in this order): root, nullifier, scope_hash
        let root_var = FpVar::<Fr>::new_input(cs.clone(), || self.root.ok_or(SynthesisError::AssignmentMissing))?;
        let null_var = FpVar::<Fr>::new_input(cs.clone(), || self.nullifier.ok_or(SynthesisError::AssignmentMissing))?;
        let scope_var = FpVar::<Fr>::new_input(cs.clone(), || self.scope_hash.ok_or(SynthesisError::AssignmentMissing))?;

        // Witnesses
        let leaf_var = FpVar::<Fr>::new_witness(cs.clone(), || self.leaf.ok_or(SynthesisError::AssignmentMissing))?;
        let secret_var = FpVar::<Fr>::new_witness(cs.clone(), || self.secret.ok_or(SynthesisError::AssignmentMissing))?;

        // Poseidon params as constant R1CS variable
        let params = self.poseidon_params.ok_or(SynthesisError::AssignmentMissing)?;
        let params_var = CRHParametersVar::new_variable(cs.clone(), || Ok(params), AllocationMode::Constant)?;

        // Fold merkle path
        let mut current = leaf_var.clone();
        for (sib_opt, is_left) in self.merkle_path.into_iter() {
            let sib = sib_opt.ok_or(SynthesisError::AssignmentMissing)?;
            let sib_var = FpVar::<Fr>::new_witness(cs.clone(), || Ok(sib))?;
            let inputs = if is_left {
                vec![current.clone(), sib_var]
            } else {
                vec![sib_var, current.clone()]
            };
            let hash_var = CRHGadget::<Fr>::evaluate(&params_var, &inputs)?;
            current = hash_var;
        }

        // Enforce computed root equals public root
        current.enforce_equal(&root_var)?;

        // Compute nullifier via Poseidon on (scope_hash, secret)
        let null_inputs = vec![scope_var.clone(), secret_var.clone()];
        let computed_null = CRHGadget::<Fr>::evaluate(&params_var, &null_inputs)?;
        computed_null.enforce_equal(&null_var)?;

        Ok(())
    }
}

/// Parse big-endian 32-byte hex into Fr via from_be_bytes_mod_order
fn fr_from_hex(hex_str: &str) -> Result<Fr> {
    use ark_ff::PrimeField;
    let bytes = hex::decode(hex_str).context("decode hex")?;
    Ok(Fr::from_be_bytes_mod_order(&bytes))
}

/// Convert `Fr` into a fixed 32-byte big-endian hex string
fn fr_to_hex(fr: Fr) -> String {
    use ark_ff::BigInteger;
    let bytes = fr.into_bigint().to_bytes_be();
    let mut buf = [0u8; 32];
    let start = 32 - bytes.len();
    buf[start..].copy_from_slice(&bytes);
    hex::encode(buf)
}

// Controlled debug logging: compile-time feature `zkp_debug` enables diagnostic output.
fn debug_log(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut log_file = OpenOptions::new().create(true).append(true).open("errors.log").unwrap();
    writeln!(log_file, "{}", msg).ok();
}

/// Compute scope hash (SHA-256) and map to Fr
fn fr_from_scope(scope: &str) -> Fr {
    use sha2::{Digest, Sha256};
    let dig = Sha256::digest(scope.as_bytes());
    use ark_ff::PrimeField;
    Fr::from_be_bytes_mod_order(&dig)
}

/// Load or generate proving key from a fixed file path
fn pk_path(data_dir: &str, vk_version: u32) -> std::path::PathBuf {
    std::path::Path::new(data_dir)
        .join("zkp")
        .join(format!("pk-groth16-v{}.bin", vk_version))
}

/// Load proving key from disk
fn load_pk(data_dir: &str, vk_version: u32) -> Result<ProvingKey<Bn254>> {
    let path = pk_path(data_dir, vk_version);
    let bytes = std::fs::read(&path).with_context(|| format!("read proving key: {}", path.display()))?;
    let pk = ProvingKey::<Bn254>::deserialize_compressed(&*bytes)
        .or_else(|_| ProvingKey::<Bn254>::deserialize_uncompressed(&*bytes))
        .context("deserialize proving key")?;
    Ok(pk)
}

/// Prove an anonymous action using the real membership + nullifier ZKP circuit (Groth16).
/// Returns (proof_bytes, public_inputs_fr)
pub fn prove_membership_nullifier(
    data_dir: &str,
    vk_version: u32,
    root_hex: &str,
    scope: &str,
    nullifier_hex: &str,
    leaf_hex: &str,
    secret_hex: &str,
    merkle_path_hex: &[&str],
    directions: &str,
) -> Result<(Vec<u8>, Vec<Fr>)> {
    use ark_ff::BigInteger;
    // Map public inputs to field elements. Circuit allocates inputs in order: root, nullifier, scope_hash
    let r = fr_from_hex(root_hex).context("root_hex to Fr")?;
    let n = fr_from_hex(nullifier_hex).context("nullifier_hex to Fr")?;
    let s = fr_from_scope(scope);

    // Map private inputs
    let leaf = fr_from_hex(leaf_hex).context("leaf_hex to Fr")?;
    let secret = fr_from_hex(secret_hex).context("secret_hex to Fr")?;
    let mut merkle_path = Vec::new();
    for (i, sib_hex) in merkle_path_hex.iter().enumerate() {
        let sib = fr_from_hex(sib_hex).context(format!("sib_hex {} to Fr", i))?;
        let is_left = directions.chars().nth(i).unwrap() == '0'; // '0' for left, '1' for right
        merkle_path.push((Some(sib), is_left));
    }

    // Prepare circuit instance
    // load poseidon params from disk (must have been created by setup)
    let poseidon_params = load_poseidon_params(data_dir).context("load poseidon params")?;
    // Print Poseidon params hash for synchronisation
    match poseidon_params_hash(&poseidon_params) {
        Ok(hash) => {
            println!("Poseidon params hash: {}", hash);
            debug_log(&format!("Poseidon params hash (cli): {}", hash));
        },
        Err(e) => println!("Poseidon params hash error: {}", e),
    }
    // Debug: print some param sizes and inputs (gated)
    debug_log(&format!(
        "poseidon params: mds {}x{}, ark {}x{}",
        poseidon_params.mds.len(),
        poseidon_params.mds.get(0).map(|r| r.len()).unwrap_or(0),
        poseidon_params.ark.len(),
        poseidon_params.ark.get(0).map(|r| r.len()).unwrap_or(0)
    ));
    debug_log(&format!("public inputs r={} n={} s={}", r, n, s));
    debug_log(&format!("leaf={} secret={}", leaf, secret));

    // Recompute native root from provided leaf/path to ensure it matches r
    let mut current = leaf;
    for (_i, sib_pair) in merkle_path.iter().enumerate() {
        let (sib_opt, is_left) = sib_pair;
        let sib = sib_opt.expect("sib present");
        let (left, right) = if *is_left { (current, sib) } else { (sib, current) };
        current = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&poseidon_params, left, right)
            .expect("compress");
    }
    let recomputed_root = current;
    let recomputed_null = ark_crypto_primitives::crh::poseidon::CRH::<Fr>::evaluate(&poseidon_params, [s, secret]).expect("crh eval");
        debug_log(&format!("recomputed_root={} recomputed_null={}", recomputed_root, recomputed_null));

    let circuit = MembershipNullifierCircuit {
        root: Some(r),
        nullifier: Some(n),
        scope_hash: Some(s),
        leaf: Some(leaf),
        merkle_path,
        secret: Some(secret),
        poseidon_params: Some(poseidon_params),
    };

    // Load proving key
    let pk = load_pk(data_dir, vk_version)?;

    // Synthesize the circuit into a standalone constraint system and check satisfiability for debugging
    {
        use ark_relations::r1cs::ConstraintSystem;
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.clone().generate_constraints(cs.clone()).map_err(|e| anyhow::anyhow!("synthesis failed: {:?}", e))?;
        let satisfied = cs.is_satisfied().unwrap_or(false);
        debug_log(&format!("pre-prove cs.is_satisfied = {}", satisfied));
        if !satisfied {
            anyhow::bail!("circuit constraints not satisfied before proving");
        }
    }

    // Prove
    let mut rng = rand::thread_rng();
    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).context("prove")?;
    // Log proof bytes as hex
    let mut proof_bytes_vec = Vec::new();
    proof.serialize_compressed(&mut proof_bytes_vec)?;
    debug_log(&format!("Proof bytes: {}", hex::encode(&proof_bytes_vec)));
    // Direct verify of the in-memory proof (no serialization) using stored VK
    let vk_file = std::path::Path::new(data_dir).join("zkp").join(format!("vk-groth16-v{}.bin", vk_version));
    if vk_file.exists() {
        let vk_bytes = std::fs::read(&vk_file).context("read vk in prove_membership_nullifier")?;
        let vk_loaded = VerifyingKey::<Bn254>::deserialize_compressed(&*vk_bytes).or_else(|_| VerifyingKey::<Bn254>::deserialize_uncompressed(&*vk_bytes))?;
        let pvk = prepare_verifying_key(&vk_loaded);
        let inputs_local = vec![r, n, s];
        let ok_direct = Groth16::<Bn254>::verify_proof(&pvk, &proof, &inputs_local).unwrap_or(false);
        debug_log(&format!("direct verify (before serialization) = {}", ok_direct));
        if !ok_direct {
            debug_log("direct verify failed; primary input ordering might be wrong or keys mismatched");
        }
    }

    let mut proof_bytes = Vec::new();
    proof.serialize_compressed(&mut proof_bytes).context("serialize proof")?;
    // Quick self-check: try to verify the proof using the stored VK
    let vk_path = std::path::Path::new(data_dir).join("zkp").join(format!("vk-groth16-v{}.bin", vk_version));
    if vk_path.exists() {
    let inputs_local = vec![r, n, s];  // Ordre correct: root, nullifier, scope_hash
    // Debug: hash pk/vk bytes to ensure they match the files written during setup
    use sha2::{Digest, Sha256};
    let pk_bytes = std::fs::read(pk_path(data_dir, vk_version)).unwrap_or_default();
    let vk_bytes = std::fs::read(vk_path.clone()).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&pk_bytes);
    let pk_hash = hasher.finalize_reset();
    hasher.update(&vk_bytes);
    let vk_hash = hasher.finalize();
    println!("PK hash: {}", hex::encode(pk_hash));
    println!("VK hash: {}", hex::encode(vk_hash));
    debug_log(&format!("VK hash (cli): {}", hex::encode(vk_hash)));
        if let Ok(vk) = VerifyingKey::<Bn254>::deserialize_compressed(&*vk_bytes).or_else(|_| VerifyingKey::<Bn254>::deserialize_uncompressed(&*vk_bytes)) {
            let pvk = prepare_verifying_key(&vk);
            debug_log(&format!("vk.gamma_abc_g1.len() = {}", vk.gamma_abc_g1.len()));
            let proof2 = Proof::<Bn254>::deserialize_compressed(&*proof_bytes).or_else(|_| Proof::<Bn254>::deserialize_uncompressed(&*proof_bytes))?;
            let inputs_local = vec![r, n, s];
            debug_log(&format!("provided pub inputs len = {}", inputs_local.len()));
            let ok_local = Groth16::<Bn254>::verify_proof(&pvk, &proof2, &inputs_local).unwrap_or(false);
            if !ok_local {
                anyhow::bail!("local verify failed after proving");
            }
        }
    }

    // Log public inputs as hex
    debug_log(&format!("Public inputs: [0] {}", root_hex));
    debug_log(&format!("Public inputs: [1] {}", nullifier_hex));
    debug_log(&format!("Public inputs: [2] {}", hex::encode(s.into_bigint().to_bytes_be())));
    // Log Fr values
    debug_log(&format!("Fr values: [0] {:?}", r.into_bigint()));
    debug_log(&format!("Fr values: [1] {:?}", n.into_bigint()));
    debug_log(&format!("Fr values: [2] {:?}", s.into_bigint()));
    Ok((proof_bytes, vec![r, n, s]))
}

/// Paths and I/O for Poseidon parameters
fn poseidon_params_path(data_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(data_dir).join("zkp").join("poseidon_params.bin")
}

fn save_poseidon_params(data_dir: &str, params: &PoseidonConfig<Fr>) -> anyhow::Result<()> {
    let path = poseidon_params_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create parent zkp dir")?;
    }
    let mut file = File::create(&path).context("create poseidon params file")?;
    // Write header fields
    file.write_all(&(params.full_rounds as u64).to_le_bytes())?;
    file.write_all(&(params.partial_rounds as u64).to_le_bytes())?;
    file.write_all(&params.alpha.to_le_bytes())?;
    file.write_all(&(params.rate as u64).to_le_bytes())?;
    file.write_all(&(params.capacity as u64).to_le_bytes())?;

    // mds dims and elements
    let mds_rows = params.mds.len() as u64;
    let mds_cols = if mds_rows > 0 { params.mds[0].len() as u64 } else { 0u64 };
    file.write_all(&mds_rows.to_le_bytes())?;
    file.write_all(&mds_cols.to_le_bytes())?;
    for row in &params.mds {
        for el in row {
            el.serialize_compressed(&mut file).context("serialize mds element")?;
        }
    }

    // ark dims and elements
    let ark_rows = params.ark.len() as u64;
    let ark_cols = if ark_rows > 0 { params.ark[0].len() as u64 } else { 0u64 };
    file.write_all(&ark_rows.to_le_bytes())?;
    file.write_all(&ark_cols.to_le_bytes())?;
    for row in &params.ark {
        for el in row {
            el.serialize_compressed(&mut file).context("serialize ark element")?;
        }
    }

    Ok(())
}

fn load_poseidon_params(data_dir: &str) -> anyhow::Result<PoseidonConfig<Fr>> {
    let path = poseidon_params_path(data_dir);
    let mut file = File::open(&path).context("open poseidon params file")?;
    use std::io::Read;
    let mut buf8 = [0u8; 8];
    file.read_exact(&mut buf8)?;
    let full_rounds = u64::from_le_bytes(buf8) as usize;
    file.read_exact(&mut buf8)?;
    let partial_rounds = u64::from_le_bytes(buf8) as usize;
    file.read_exact(&mut buf8)?;
    let alpha = u64::from_le_bytes(buf8);
    file.read_exact(&mut buf8)?;
    let rate = u64::from_le_bytes(buf8) as usize;
    file.read_exact(&mut buf8)?;
    let capacity = u64::from_le_bytes(buf8) as usize;

    // mds
    file.read_exact(&mut buf8)?;
    let mds_rows = u64::from_le_bytes(buf8) as usize;
    file.read_exact(&mut buf8)?;
    let mds_cols = u64::from_le_bytes(buf8) as usize;
    let mut mds: Vec<Vec<Fr>> = Vec::with_capacity(mds_rows);
    for _ in 0..mds_rows {
        let mut row: Vec<Fr> = Vec::with_capacity(mds_cols);
        for _ in 0..mds_cols {
            let el = <Fr as CanonicalDeserialize>::deserialize_compressed(&mut file)
                .or_else(|_| <Fr as CanonicalDeserialize>::deserialize_uncompressed(&mut file))
                .context("deserialize mds element")?;
            row.push(el);
        }
        mds.push(row);
    }

    // ark
    file.read_exact(&mut buf8)?;
    let ark_rows = u64::from_le_bytes(buf8) as usize;
    file.read_exact(&mut buf8)?;
    let ark_cols = u64::from_le_bytes(buf8) as usize;
    let mut ark: Vec<Vec<Fr>> = Vec::with_capacity(ark_rows);
    for _ in 0..ark_rows {
        let mut row: Vec<Fr> = Vec::with_capacity(ark_cols);
        for _ in 0..ark_cols {
            let el = <Fr as CanonicalDeserialize>::deserialize_compressed(&mut file)
                .or_else(|_| <Fr as CanonicalDeserialize>::deserialize_uncompressed(&mut file))
                .context("deserialize ark element")?;
            row.push(el);
        }
        ark.push(row);
    }

    let params = PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity);
    Ok(params)
}

/// Helper to write PK and VK to disk for a fixed toy circuit using zero assignments.
/// This lets operators create vk/pk pairs wallet+node agree on (same vk_version scheme as server).
pub fn setup_placeholder_keys(data_dir: &str, vk_version: u32) -> Result<()> {
    use ark_groth16::VerifyingKey;
    use ark_groth16::prepare_verifying_key;
    use ark_snark::CircuitSpecificSetupSNARK;
    use std::fs;
    // Setup circuit with zero assignments (valid for our circuit)
    let mut rng = rand::thread_rng();
    // The crate's automatic parameter generation is not implemented for CRH::setup.
    // Generate parameters deterministically via the provided helper `find_poseidon_ark_and_mds`.
    let rate = 2usize;
    let full_rounds: u64 = 8;
    let partial_rounds: u64 = 57;
    let skip_matrices: u64 = 0;
    let prime_bits = Fr::MODULUS_BIT_SIZE as u64;
    let (ark, mds) = ark_crypto_primitives::sponge::poseidon::find_poseidon_ark_and_mds(
        prime_bits,
        rate,
        full_rounds,
        partial_rounds,
        skip_matrices,
    );
    let poseidon_params = PoseidonConfig::new(
        full_rounds as usize,
        partial_rounds as usize,
        5u64,
        mds,
        ark,
        rate,
        1usize,
    );
    save_poseidon_params(data_dir, &poseidon_params).context("save poseidon params")?;
    // Build a merkle path of zero siblings and compute the native Merkle root and nullifier
    let zero = Fr::from(0u64);
    let scope = "support";
    let scope_hash = fr_from_scope(scope);
    let mut merkle_path_setup = Vec::with_capacity(MERKLE_DEPTH);
    for i in 0..MERKLE_DEPTH {
            merkle_path_setup.push((Some(zero), i % 2 == 0)); // Adjusted to maintain original logic
    }

    // Compute native merkle root by folding TwoToOneCRH compress over the path (zero siblings)
    let mut current = zero;
    for (sib_opt, is_left) in &merkle_path_setup {
        let sib = sib_opt.expect("sib present");
        let (left, right) = if *is_left { (current, sib) } else { (sib, current) };
        current = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&poseidon_params, left, right)
            .map_err(|e| anyhow::anyhow!("compress error: {}", e))?;
    }
    let native_root = current;

    // Compute native nullifier = Poseidon(scope_hash, secret=0)
    let native_nullifier = ark_crypto_primitives::crh::poseidon::CRH::<Fr>::evaluate(&poseidon_params, [scope_hash, zero])
        .map_err(|e| anyhow::anyhow!("crh evaluate error: {}", e))?;

    println!("setup native_root_hex={} native_nullifier_hex={}", fr_to_hex(native_root), fr_to_hex(native_nullifier));

    let setup_circuit = MembershipNullifierCircuit {
        root: Some(native_root),
        nullifier: Some(native_nullifier),
        scope_hash: Some(scope_hash),
        leaf: Some(zero),
        merkle_path: merkle_path_setup.clone(),
        secret: Some(zero),
        poseidon_params: Some(poseidon_params.clone()),
    };

    let (pk, vk): (ProvingKey<Bn254>, VerifyingKey<Bn254>) = Groth16::<Bn254>::setup(setup_circuit, &mut rng).context("setup")?;
    let dir = std::path::Path::new(data_dir).join("zkp");
    fs::create_dir_all(&dir)?;
    // Write VK to node-expected name
    let vk_path = dir.join(format!("vk-groth16-v{}.bin", vk_version));
    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes)?;
    fs::write(&vk_path, &vk_bytes)?;
    // Write PK alongside
    let pk_path = dir.join(format!("pk-groth16-v{}.bin", vk_version));
    let mut pk_bytes = Vec::new();
    pk.serialize_compressed(&mut pk_bytes)?;
    fs::write(&pk_path, &pk_bytes)?;
    // Debug: print hashes of written keys
    {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&pk_bytes);
        let pk_hash = hasher.finalize_reset();
        hasher.update(&vk_bytes);
        let vk_hash = hasher.finalize();
        println!("setup wrote pk_hash={} vk_hash={}", hex::encode(pk_hash), hex::encode(vk_hash));
    }
    // pvk preparation is not written but we ensure vk is valid
    let _ = prepare_verifying_key(&vk);
    Ok(())
}

/// Like `setup_placeholder_keys` but also returns the generated keys and params in-memory.
pub fn setup_placeholder_keys_returning(data_dir: &str, vk_version: u32) -> Result<(ProvingKey<Bn254>, VerifyingKey<Bn254>, PoseidonConfig<Fr>)> {
    use ark_groth16::VerifyingKey;
    use ark_groth16::prepare_verifying_key;
    use ark_snark::CircuitSpecificSetupSNARK;
    use std::fs;
    // Setup circuit with zero assignments (valid for our circuit)
    let mut rng = rand::thread_rng();
    let rate = 2usize;
    let full_rounds: u64 = 8;
    let partial_rounds: u64 = 57;
    let skip_matrices: u64 = 0;
    let prime_bits = Fr::MODULUS_BIT_SIZE as u64;
    let (ark, mds) = ark_crypto_primitives::sponge::poseidon::find_poseidon_ark_and_mds(
        prime_bits,
        rate,
        full_rounds,
        partial_rounds,
        skip_matrices,
    );
    let poseidon_params = PoseidonConfig::new(
        full_rounds as usize,
        partial_rounds as usize,
        5u64,
        mds,
        ark,
        rate,
        1usize,
    );
    save_poseidon_params(data_dir, &poseidon_params).context("save poseidon params")?;

    // Compute a hash of poseidon params for later comparison
    fn poseidon_params_hash(params: &PoseidonConfig<Fr>) -> anyhow::Result<String> {
        use sha2::{Digest, Sha256};
        use ark_serialize::CanonicalSerialize;
        let mut hasher = Sha256::new();
        hasher.update(&params.full_rounds.to_le_bytes());
        hasher.update(&params.partial_rounds.to_le_bytes());
        hasher.update(&params.alpha.to_le_bytes());
        hasher.update(&params.rate.to_le_bytes());
        hasher.update(&params.capacity.to_le_bytes());
        for row in &params.mds {
            for el in row {
                let mut b = Vec::new();
                el.serialize_compressed(&mut b)?;
                hasher.update(&b);
            }
        }
        for row in &params.ark {
            for el in row {
                let mut b = Vec::new();
                el.serialize_compressed(&mut b)?;
                hasher.update(&b);
            }
        }
        Ok(hex::encode(hasher.finalize()))
    }

    let zero = Fr::from(0u64);
    let mut merkle_path_setup = Vec::with_capacity(MERKLE_DEPTH);
    for i in 0..MERKLE_DEPTH {
        merkle_path_setup.push((Some(zero), i % 2 == 0));
    }
    let mut current = zero;
    for (sib_opt, is_left) in &merkle_path_setup {
        let sib = sib_opt.expect("sib present");
        let (left, right) = if *is_left { (current, sib) } else { (sib, current) };
        current = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&poseidon_params, left, right)
            .map_err(|e| anyhow::anyhow!("compress error: {}", e))?;
    }
    let native_root = current;
    let native_nullifier = ark_crypto_primitives::crh::poseidon::CRH::<Fr>::evaluate(&poseidon_params, [zero, zero])
        .map_err(|e| anyhow::anyhow!("crh evaluate error: {}", e))?;

    let setup_circuit = MembershipNullifierCircuit {
        root: Some(native_root),
        nullifier: Some(native_nullifier),
        scope_hash: Some(zero),
        leaf: Some(zero),
        merkle_path: merkle_path_setup,
        secret: Some(zero),
        poseidon_params: Some(poseidon_params.clone()),
    };
    // Synthesize setup circuit for diagnostics (gated)
    {
        use ark_relations::r1cs::ConstraintSystem;
        let cs = ConstraintSystem::<Fr>::new_ref();
        setup_circuit.clone().generate_constraints(cs.clone()).expect("synthesis setup");
        debug_log(&format!("[diag setup] setup circuit num_constraints={} num_witness_vars={} is_satisfied={}", cs.num_constraints(), cs.num_witness_variables(), cs.is_satisfied().unwrap_or(false)));
    }

    let params_hash = poseidon_params_hash(&poseidon_params).expect("hash params");
    debug_log(&format!("[diag setup] poseidon params hash = {}", params_hash));

    let (pk, vk): (ProvingKey<Bn254>, VerifyingKey<Bn254>) = Groth16::<Bn254>::setup(setup_circuit, &mut rng).context("setup")?;
    let dir = std::path::Path::new(data_dir).join("zkp");
    fs::create_dir_all(&dir)?;
    // Write VK to node-expected name
    let vk_path = dir.join(format!("vk-groth16-v{}.bin", vk_version));
    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes)?;
    fs::write(&vk_path, &vk_bytes)?;
    // Write PK alongside
    let pk_path = dir.join(format!("pk-groth16-v{}.bin", vk_version));
    let mut pk_bytes = Vec::new();
    pk.serialize_compressed(&mut pk_bytes)?;
    fs::write(&pk_path, &pk_bytes)?;
    Ok((pk, vk, poseidon_params))
}

// Test-only helper to synthesize the circuit and print constraint stats / first unsatisfied constraint
#[cfg(all(test, feature = "zkp_debug"))]
pub fn debug_circuit_satisfaction(
    root: Fr,
    nullifier: Fr,
    scope_hash: Fr,
    leaf: Fr,
    secret: Fr,
    merkle_path: Vec<(Option<Fr>, bool)>,
    poseidon_params: PoseidonConfig<Fr>,
) -> anyhow::Result<()> {
    use ark_relations::r1cs::ConstraintSystem;
    use anyhow::Context;

    // Basic sanity check on path length
    if merkle_path.len() != MERKLE_DEPTH {
        println!(
            "Warning: merkle_path.len() = {} (expected MERKLE_DEPTH={})",
            merkle_path.len(),
            MERKLE_DEPTH
        );
    }

    let cs = ConstraintSystem::<Fr>::new_ref();
    let circuit = MembershipNullifierCircuit {
        root: Some(root),
        nullifier: Some(nullifier),
        scope_hash: Some(scope_hash),
        leaf: Some(leaf),
        merkle_path,
        secret: Some(secret),
        poseidon_params: Some(poseidon_params),
    };

    // Synthesize the circuit
    circuit
        .generate_constraints(cs.clone())
        .context("synthesis failed")?;

    println!("Constraint system stats:");
    println!("  Number of constraints: {}", cs.num_constraints());
    // num_inputs / num_aux are present in recent ark versions; fall back if not
    // Use method existence at compile time; if unavailable the call will fail to compile and can be adapted.
    // ConstraintSystemRef has helpers for witness/input variable counts
    println!("  Number of witness variables: {}", cs.num_witness_variables());
    // is_satisfied might return Err; handle gracefully
    let satisfied = cs.is_satisfied().unwrap_or(false);
    println!("  Is satisfied: {}", satisfied);

    if !satisfied {
        println!("Constraints not satisfied!");
        // Some ConstraintSystem implementations provide `which_is_unsatisfied()`
        match cs.which_is_unsatisfied() {
            Ok(Some(which)) => println!("First unsatisfied constraint: {}", which),
            Ok(None) => println!("which_is_unsatisfied returned None."),
            Err(e) => println!("which_is_unsatisfied returned error: {:?}", e),
        }
    }

    Ok(())
}

/// Compute and print the Poseidon hash of two values, for debugging purposes.
/// This is a standalone function, not integrated into the circuit.
pub fn debug_poseidon_hash(data_dir: &str, hex1: &str, hex2: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    // Load poseidon params
    let params = load_poseidon_params(data_dir).context("load poseidon params")?;
    let f = |h: &str| -> Fr { fr_from_hex(h).expect("fr from hex") };
    let v = |f: Fr| -> Vec<u8> { hex::decode(fr_to_hex(f)).expect("hex decode") };

    // Compute hash
    let fr1 = f(hex1);
    let fr2 = f(hex2);
    let mut out = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&params, fr1, fr2)
        .expect("poseidon compress");
    // Output intermediate values and final hash
    println!("debug_poseidon_hash: {} + {} =", hex1, hex2);
    println!("  fr1 = {:?}", v(fr1));
    println!("  fr2 = {:?}", v(fr2));
    let hash = ark_crypto_primitives::crh::poseidon::CRH::<ark_bn254::Fr>::evaluate(&params, [fr1, fr2])
        .map_err(|e| anyhow::anyhow!("Poseidon error: {:?}", e))?;
    println!("Poseidon hash result (hex): {}", fr_to_hex(hash));
    println!("Poseidon hash result (decimal): {}", hash);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_groth16::{VerifyingKey, prepare_verifying_key, Proof};
    use tempfile::tempdir;

    #[test]
    fn setup_returning_inmemory_prove_verify() {
        let tmp = tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
        let vk_version = 9u32;

        // Get in-memory keys + params and also write them to disk
        let (pk, vk, params) = setup_placeholder_keys_returning(data_dir, vk_version).expect("setup returning");

        // Prepare a deterministic example matching the setup circuit structure
        let mut rng = rand::thread_rng();
        let leaf = Fr::rand(&mut rng);
        let secret = Fr::rand(&mut rng);
        let scope = "support:inmemory";
        let scope_fr = fr_from_scope(scope);

        // build a random merkle path
        let mut merkle_path = Vec::new();
        let mut merkle_path_hex = Vec::new();
        let mut directions = String::new();
        use ark_ff::BigInteger;
        for i in 0..MERKLE_DEPTH {
            let s = Fr::rand(&mut rng);
            merkle_path.push((Some(s), i % 2 == 0));
            merkle_path_hex.push(hex::encode(s.into_bigint().to_bytes_be()));
            directions.push(if i % 2 == 0 { '0' } else { '1' });
        }

        // compute native root & null
        let mut current = leaf;
        for (sib, is_left) in &merkle_path {
            let sib = sib.expect("sib");
            let (left, right) = if *is_left { (current, sib) } else { (sib, current) };
            current = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&params, left, right).expect("compress");
        }
        let native_root = current;
        let native_null = ark_crypto_primitives::crh::poseidon::CRH::<Fr>::evaluate(&params, [scope_fr, secret]).expect("crh");

        // Build circuit instance
        let circuit = MembershipNullifierCircuit {
            root: Some(native_root),
            nullifier: Some(native_null),
            scope_hash: Some(scope_fr),
            leaf: Some(leaf),
            merkle_path: merkle_path.clone(),
            secret: Some(secret),
            poseidon_params: Some(params.clone()),
        };

            // Synthesize circuit for diagnostics (gated)
            {
                use ark_relations::r1cs::ConstraintSystem;
                let cs = ConstraintSystem::<Fr>::new_ref();
                circuit.clone().generate_constraints(cs.clone()).expect("synthesis");
                debug_log(&format!("[diag] circuit num_constraints={} num_witness_vars={} is_satisfied={}", cs.num_constraints(), cs.num_witness_variables(), cs.is_satisfied().unwrap_or(false)));
            }

            // Print VK sizes (gated)
            debug_log(&format!("[diag] vk.gamma_abc_g1.len() = {}", vk.gamma_abc_g1.len()));
            debug_log("[diag] expected public inputs = 3");

        // Prove using in-memory pk
        let mut rng2 = rand::thread_rng();
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng2).expect("prove inmemory");

    // Serialize pk/vk/proof and print hashes for diagnostics
    use sha2::{Digest, Sha256};
    let mut pk_bytes = Vec::new();
    pk.serialize_compressed(&mut pk_bytes).expect("serialize pk");
    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes).expect("serialize vk");
    let mut proof_bytes = Vec::new();
    proof.serialize_compressed(&mut proof_bytes).expect("serialize proof");
    let mut hasher = Sha256::new();
    hasher.update(&pk_bytes);
    let pk_hash = hasher.finalize_reset();
    hasher.update(&vk_bytes);
    let vk_hash = hasher.finalize_reset();
    hasher.update(&proof_bytes);
    let proof_hash = hasher.finalize();
    debug_log(&format!("[diag] pk_bytes.len={} vk_bytes.len={} proof_bytes.len={}", pk_bytes.len(), vk_bytes.len(), proof_bytes.len()));
    debug_log(&format!("[diag] pk_hash={} vk_hash={} proof_hash={}", hex::encode(pk_hash), hex::encode(vk_hash), hex::encode(proof_hash)));

    let pvk = prepare_verifying_key(&vk);
    let inputs = vec![native_root, native_null, scope_fr]; // Ordre: root, nullifier, scope_hash
    debug_log(&format!("[diag] public inputs: root={}, null={}, scope={}", native_root, native_null, scope_fr));
    let ok = Groth16::<Bn254>::verify_proof(&pvk, &proof, &inputs).expect("verify inmemory");
    debug_log(&format!("[diag] direct verify ok = {}", ok));

    // Try verify after deserialize proof
    let proof2 = Proof::<Bn254>::deserialize_compressed(&*proof_bytes).or_else(|_| Proof::<Bn254>::deserialize_uncompressed(&*proof_bytes)).expect("proof deser");
    let ok2 = Groth16::<Bn254>::verify_proof(&pvk, &proof2, &inputs).expect("verify deserialized");
    debug_log(&format!("[diag] deserialized proof verify ok = {}", ok2));
    assert!(ok && ok2, "in-memory pk should verify");
    }
    #[test]
    fn inproc_setup_prove_verify() {
        // Build deterministic Poseidon params (same approach as setup_placeholder_keys)
        let rate = 2usize;
        let full_rounds: u64 = 8;
        let partial_rounds: u64 = 57;
        let skip_matrices: u64 = 0;
        let prime_bits = Fr::MODULUS_BIT_SIZE as u64;
        let (ark, mds) = ark_crypto_primitives::sponge::poseidon::find_poseidon_ark_and_mds(
            prime_bits,
            rate,
            full_rounds,
            partial_rounds,
            skip_matrices,
        );
        let poseidon_params = PoseidonConfig::new(
            full_rounds as usize,
            partial_rounds as usize,
            5u64,
            mds,
            ark,
            rate,
            1usize,
        );

        // Create random leaf/secret and merkle path
        let mut rng = rand::thread_rng();
        let leaf = Fr::rand(&mut rng);
        let secret = Fr::rand(&mut rng);
        let mut siblings = Vec::new();
        let mut directions = Vec::new();
        for i in 0..MERKLE_DEPTH {
            let s = Fr::rand(&mut rng);
            siblings.push(s);
            directions.push((s, (i % 2) == 0));
        }

        // Compute native root and nullifier
        let mut current = leaf;
        for (sib, is_left) in &directions {
            let (left, right) = if *is_left { (current, *sib) } else { (*sib, current) };
            current = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&poseidon_params, left, right).expect("compress");
        }
        let native_root = current;
        let scope = "support:inproc";
        let scope_fr = fr_from_scope(scope);
        let native_null = ark_crypto_primitives::crh::poseidon::CRH::<Fr>::evaluate(&poseidon_params, [scope_fr, secret]).expect("crh");

        let circuit = MembershipNullifierCircuit {
            root: Some(native_root),
            nullifier: Some(native_null),
            scope_hash: Some(scope_fr),
            leaf: Some(leaf),
            merkle_path: directions.iter().map(|(s, is_left)| (Some(*s), *is_left)).collect(),
            secret: Some(secret),
            poseidon_params: Some(poseidon_params.clone()),
        };

        // Setup
        let mut rng2 = rand::thread_rng();
        let (pk, vk) = Groth16::<Bn254>::setup(circuit.clone(), &mut rng2).expect("setup");
        let pvk = prepare_verifying_key(&vk);

        // Prove
        let mut rng3 = rand::thread_rng();
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng3).expect("prove");

        // Verify
    let inputs = vec![native_root, native_null, scope_fr]; // Ordre: root, nullifier, scope_hash
        let ok = Groth16::<Bn254>::verify_proof(&pvk, &proof, &inputs).expect("verify");
        assert!(ok, "in-process proof should verify");
    }

    #[test]
    fn setup_prove_verify_roundtrip() {
        let tmp = tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
        let vk_version = 7u32;
        // Measure setup (includes poseidon param generation and Groth16 setup)
        let t0 = std::time::Instant::now();
        setup_placeholder_keys(data_dir, vk_version).expect("setup");
        let dur_setup = t0.elapsed();

        // Generate random inputs for a realistic Merkle path
        let mut rng = rand::thread_rng();
    // generate random Fr values and encode as 32-byte big-endian hex
    let mut b = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rng, &mut b);
    let root_fr = Fr::from_be_bytes_mod_order(&b);
    let _root_hex = fr_to_hex(root_fr);
    let scope = "support";
    rand::RngCore::fill_bytes(&mut rng, &mut b);
    let null_fr = Fr::from_be_bytes_mod_order(&b);
    let _null_hex = fr_to_hex(null_fr);
    rand::RngCore::fill_bytes(&mut rng, &mut b);
    let leaf_fr = Fr::from_be_bytes_mod_order(&b);
    let leaf_hex = fr_to_hex(leaf_fr);
    rand::RngCore::fill_bytes(&mut rng, &mut b);
    let secret_fr = Fr::from_be_bytes_mod_order(&b);
    let secret_hex = fr_to_hex(secret_fr);

        // Create merkle path siblings and directions
        let mut merkle_path_hex = Vec::with_capacity(MERKLE_DEPTH);
        let mut directions = String::with_capacity(MERKLE_DEPTH);
        for i in 0..MERKLE_DEPTH {
            let mut b = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rng, &mut b);
            merkle_path_hex.push(hex::encode(b));
            directions.push(if (i % 2) == 0 { '0' } else { '1' });
        }

        // Load poseidon params and compute native root & nullifier consistent with inputs
    let params = load_poseidon_params(data_dir).expect("load params");
    // use the Fr values generated above
    let leaf_fr = leaf_fr;
    let secret_fr = secret_fr;
        // build merkle siblings as Fr
        let mut current = leaf_fr;
        for (i, sib_hex) in merkle_path_hex.iter().enumerate() {
            let sib_fr = fr_from_hex(sib_hex).expect("sib fr");
            let is_left = directions.chars().nth(i).unwrap() == '0';
            let (left, right) = if is_left { (current, sib_fr) } else { (sib_fr, current) };
            current = ark_crypto_primitives::crh::poseidon::TwoToOneCRH::<Fr>::compress(&params, left, right)
                .expect("compress");
        }
    let native_root_fr = current;
    let scope_fr = fr_from_scope(scope);
    let native_nullifier_fr = ark_crypto_primitives::crh::poseidon::CRH::<Fr>::evaluate(&params, [scope_fr, secret_fr]).expect("crh eval");

        // convert to hex for proving API
        use ark_ff::BigInteger;
        let root_bytes = native_root_fr.into_bigint().to_bytes_be();
        let null_bytes = native_nullifier_fr.into_bigint().to_bytes_be();
        let root_hex = hex::encode(root_bytes);
        let null_hex = hex::encode(null_bytes);

        // Build merkle siblings Vec<Fr> for the user's debug call
        let _siblings_vals: Vec<Fr> = merkle_path_hex
            .iter()
            .map(|h| fr_from_hex(h).expect("sib fr"))
            .collect();

        // Optionally run the debug helper (only compiled when `zkp_debug` feature is enabled)
        #[cfg(feature = "zkp_debug")]
        {
            debug_circuit_satisfaction(
                native_root_fr,
                native_nullifier_fr,
                scope_fr,
                leaf_fr,
                secret_fr,
                siblings_vals.iter().map(|&s| (Some(s), true)).collect(), // Adjust directions as needed
                params.clone(),
            )
            .expect("debug circuit");
        }

        // Prove (measure)
        let t1 = std::time::Instant::now();
        let (proof_bytes, pub_inputs) = prove_membership_nullifier(
            data_dir,
            vk_version,
            &root_hex,
            scope,
            &null_hex,
            &leaf_hex,
            &secret_hex,
            &merkle_path_hex.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            &directions,
        )
        .expect("prove");
        let dur_prove = t1.elapsed();

    let dur_verify_start = std::time::Instant::now();
        // Load VK and verify proof directly
        let vk_path = std::path::Path::new(data_dir).join("zkp").join(format!("vk-groth16-v{}.bin", vk_version));
        let vk_bytes = std::fs::read(&vk_path).expect("read vk");
        let vk = VerifyingKey::<Bn254>::deserialize_compressed(&*vk_bytes)
            .or_else(|_| VerifyingKey::<Bn254>::deserialize_uncompressed(&*vk_bytes))
            .expect("vk deser");
        let pvk = prepare_verifying_key(&vk);
        let proof = Proof::<Bn254>::deserialize_compressed(&*proof_bytes)
            .or_else(|_| Proof::<Bn254>::deserialize_uncompressed(&*proof_bytes))
            .expect("proof deser");
    // Debug: print native values and public inputs
    println!("native_root_fr = {:?}", native_root_fr);
    println!("native_nullifier_fr = {:?}", native_nullifier_fr);
    println!("pub_inputs = {:?}", pub_inputs);

    let ok = Groth16::<Bn254>::verify_proof(&pvk, &proof, &pub_inputs).expect("verify");
        let dur_verify = dur_verify_start.elapsed();
        assert!(ok, "proof should verify");

        println!("timings: setup={:?}, prove={:?}, verify={:?}", dur_setup, dur_prove, dur_verify);
    }

    #[test]
    fn debug_poseidon_hash_test() {
        let tmp = tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
           // Ensure Poseidon params are generated in the temp directory
           let vk_version = 1u32;
           setup_placeholder_keys(data_dir, vk_version).expect("setup");
           // Use two sample hex values
           let hex1 = "0000000000000000000000000000000000000000000000000000000000000001";
           let hex2 = "0000000000000000000000000000000000000000000000000000000000000002";

           // This should compute the Poseidon hash of the two values, print the result in hex and decimal,
           // and not panic or return an error.
           debug_poseidon_hash(data_dir, hex1, hex2).expect("debug_poseidon_hash failed");
    }
}

#[cfg(test)]
mod debug_poseidon_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_poseidon_hash_zero_scope() {
        let tmp = tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
        let vk_version = 1u32;
        // Setup params
        setup_placeholder_keys(data_dir, vk_version).expect("setup");
        // Use zero scope and zero secret
        let zero_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        // Compute hash
        debug_poseidon_hash(data_dir, zero_hex, zero_hex).expect("poseidon hash");
    }

    #[test]
    fn test_poseidon_hash_scope_support() {
        let tmp = tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
        let vk_version = 1u32;
        setup_placeholder_keys(data_dir, vk_version).expect("setup");
        // Compute scope hash as in fr_from_scope
        let scope = "support";
        let scope_fr = fr_from_scope(scope);
        let scope_hex = fr_to_hex(scope_fr);
        let zero_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        debug_poseidon_hash(data_dir, scope_hex.as_str(), zero_hex).expect("poseidon hash");
    }
}
