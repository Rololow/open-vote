#![allow(dead_code)]

// Minimal Groth16 verifier binding for BN254. Feature-gated behind `zkp_groth16`.

#[cfg(feature = "zkp_groth16")]
use ark_ff::PrimeField;

#[cfg(feature = "zkp_groth16")]
use ark_bn254::{Bn254, Fr};
#[cfg(feature = "zkp_groth16")]
use ark_groth16::{prepare_verifying_key, Groth16, Proof, VerifyingKey};
#[cfg(feature = "zkp_groth16")]
use ark_serialize::CanonicalDeserialize;

#[cfg(feature = "zkp_groth16")]
use anyhow::{Context, Result};

#[cfg(feature = "zkp_groth16")]
pub fn vk_path_for_version(data_dir: &str, vk_version: u32) -> std::path::PathBuf {
    // Store verifying keys under data_dir/zkp/vk-groth16-v{n}.bin
    std::path::Path::new(data_dir)
        .join("zkp")
        .join(format!("vk-groth16-v{}.bin", vk_version))
}

#[cfg(feature = "zkp_groth16")]
pub fn load_vk(data_dir: &str, vk_version: u32) -> Result<VerifyingKey<Bn254>> {
    let path = vk_path_for_version(data_dir, vk_version);
    let bytes = std::fs::read(&path)
        .with_context(|| format!("read verifying key: {}", path.display()))?;
    // Affiche le hash VK pour synchronisation
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let vk_hash = hasher.finalize();
    // Log VK hash to errors.log for cross-check
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
            writeln!(log_file, "VK hash (serveur): {}", hex::encode(vk_hash)).ok();
        }
    }
    println!("VK hash (serveur): {}", hex::encode(vk_hash));
    let vk = VerifyingKey::<Bn254>::deserialize_compressed(&*bytes)
        .or_else(|_| VerifyingKey::<Bn254>::deserialize_uncompressed(&*bytes))
        .context("deserialize verifying key")?;
    Ok(vk)
}

#[cfg(feature = "zkp_groth16")]
pub fn poseidon_params_path(data_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(data_dir).join("zkp").join("poseidon_params.bin")
}

#[cfg(feature = "zkp_groth16")]
pub fn print_poseidon_params_hash(data_dir: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    use ark_bn254::Fr;
    use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
    use ark_serialize::CanonicalSerialize;
    use std::io::Read;
    let path = poseidon_params_path(data_dir);
    let mut file = std::fs::File::open(&path)
        .with_context(|| format!("open poseidon params file: {}", path.display()))?;
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
            // Deserialize using ark-serialize so we match wallet-cli's
            // serialize_compressed output. Try compressed first, then
            // fall back to uncompressed.
            let el = <Fr as CanonicalDeserialize>::deserialize_compressed(&mut file)
                .or_else(|_| <Fr as CanonicalDeserialize>::deserialize_uncompressed(&mut file))
                .context("deserialize poseidon mds element")?;
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
            // Deserialize using ark-serialize so we match wallet-cli's
            // serialize_compressed output. Try compressed first, then
            // fall back to uncompressed.
            let el = <Fr as CanonicalDeserialize>::deserialize_compressed(&mut file)
                .or_else(|_| <Fr as CanonicalDeserialize>::deserialize_uncompressed(&mut file))
                .context("deserialize poseidon ark element")?;
            row.push(el);
        }
        ark.push(row);
    }

    let params = PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity);
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
    let hash = hasher.finalize();
    // Log Poseidon params hash to errors.log for cross-check
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
            writeln!(log_file, "Poseidon params hash (serveur): {}", hex::encode(hash)).ok();
        }
    }
    println!("Poseidon params hash (serveur): {}", hex::encode(hash));
    Ok(())
}

#[cfg(feature = "zkp_groth16")]
pub fn load_poseidon_params(_data_dir: &str) -> Result<ark_crypto_primitives::sponge::poseidon::PoseidonConfig<ark_bn254::Fr>> {
    use ark_bn254::Fr;
    use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
    use std::io::Read;

    // First, try to load the binary poseidon params file produced by wallet-cli
    let bin_path = poseidon_params_path(_data_dir);
    if bin_path.exists() {
        let mut file = std::fs::File::open(&bin_path)
            .with_context(|| format!("open poseidon params file: {}", bin_path.display()))?;
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
                    .context("deserialize poseidon mds element")?;
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
                    .context("deserialize poseidon ark element")?;
                row.push(el);
            }
            ark.push(row);
        }

        return Ok(PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity));
    }

    // If binary params not found, fall back to config/poseidon_config.json (same behavior as before)
    let config_path = std::path::Path::new("config/poseidon_config.json");
    if config_path.exists() {
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
        return Ok(PoseidonConfig::new(
            full_rounds as usize,
            partial_rounds as usize,
            alpha,
            mds,
            ark,
            rate,
            capacity,
        ));
    }

    // Final fallback: generate parameters with reasonable defaults using arkworks helper
    let rate = 2usize;
    let full_rounds = 8usize;
    let partial_rounds = 57usize;
    let skip_matrices = 0u64;
    let prime_bits = Fr::MODULUS_BIT_SIZE as u64;
    let alpha = 5u64;
    let capacity = 1usize;
    let (ark, mds) = ark_crypto_primitives::sponge::poseidon::find_poseidon_ark_and_mds(
        prime_bits,
        rate,
        full_rounds as u64,
        partial_rounds as u64,
        skip_matrices,
    );
    Ok(PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity))
}

#[cfg(feature = "zkp_groth16")]
pub fn verify_groth16_bn254(
    data_dir: &str,
    vk_version: u32,
    proof_bytes: &[u8],
    public_inputs_fr: &[Fr],
    poseidon_params: &std::sync::Arc<ark_crypto_primitives::sponge::poseidon::PoseidonConfig<ark_bn254::Fr>>,
) -> Result<bool> {
    // If mock mode is enabled and the VK file begins with a magic header, short-circuit
    // cryptographic verification with a simple check to allow integration tests without
    // real proving/verifying keys.
    #[cfg(feature = "zkp_mock")]
    {
        use std::io::Read;
        let path = vk_path_for_version(data_dir, vk_version);
        if let Ok(mut f) = std::fs::File::open(&path) {
            let mut magic = [0u8; 6];
            if let Ok(n) = f.read(&mut magic) {
                if n == 6 && &magic == b"MOCKVK" {
                    // Accept only a specific proof payload to simulate success
                    return Ok(proof_bytes == b"OK");
                }
            }
        }
    }

    // Debug log: about to start Groth16 verification
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
            writeln!(log_file, "[verify_zkp_handler] Starting Groth16 verification").ok();
        }
    }
    let vk = match load_vk(data_dir, vk_version) {
        Ok(vk) => vk,
        Err(e) => {
            // Log error
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
                writeln!(log_file, "[verify_zkp_handler] VK load error: {}", e).ok();
            }
            return Err(e);
        }
    };
    let pvk = prepare_verifying_key(&vk);
    let proof = match Proof::<Bn254>::deserialize_compressed(proof_bytes)
        .or_else(|_| Proof::<Bn254>::deserialize_uncompressed(proof_bytes)) {
        Ok(proof) => proof,
        Err(e) => {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
                writeln!(log_file, "[verify_zkp_handler] Proof deserialization error: {}", e).ok();
            }
            return Err(anyhow::Error::from(e));
        }
    };
    let _ = poseidon_params; // placeholder to avoid unused warning
    let ok = match Groth16::<Bn254>::verify_proof(&pvk, &proof, public_inputs_fr) {
        Ok(ok) => {
            // Log success
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
                writeln!(log_file, "[verify_zkp_handler] Groth16 verification result: {}", ok).ok();
            }
            ok
        },
        Err(e) => {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut log_file) = OpenOptions::new().create(true).append(true).open("errors.log") {
                writeln!(log_file, "[verify_zkp_handler] Groth16 verification error: {}", e).ok();
            }
            return Err(anyhow::Error::from(e));
        }
    };
    Ok(ok)
}

// Helpers to map hex/string inputs to field elements (Fr)
#[cfg(feature = "zkp_groth16")]
pub fn fr_from_be_bytes(bytes: &[u8]) -> Option<Fr> {
    use ark_ff::PrimeField;
    // Interpret big-endian as a big integer modulo field modulus
    // ark-ff provides from_be_bytes_mod_order for this purpose
    Some(Fr::from_be_bytes_mod_order(bytes))
}

#[cfg(all(test, feature = "zkp_groth16"))]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    // This is a smoke test that exercises file loading and failure path of proof deserialization.
    // It writes a small dummy file as a "vk" and ensures load_vk returns an error; then
    // it verifies that verify_groth16_bn254 returns an error for bad proof bytes when a real vk is absent.
    // When zkp_mock feature is used in the future, we can inject a known-good vk fixture.
    #[test]
    fn load_vk_fails_on_nonexistent_or_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
        // No file -> error
        let err1 = load_vk(data_dir, 1).err().unwrap();
        assert!(format!("{}", err1).contains("read verifying key"));

        // Create a bogus file that won't deserialize
        let vk_path = super::vk_path_for_version(data_dir, 1);
        fs::create_dir_all(vk_path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(&vk_path).unwrap();
        f.write_all(b"not a real vk").unwrap();
        drop(f);
        let err2 = load_vk(data_dir, 1).err().unwrap();
        assert!(format!("{}", err2).contains("deserialize verifying key"));
    }

    #[test]
    fn verify_errors_on_invalid_proof() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().to_str().unwrap();
        // Write an invalid vk to get past file read; still deserialization should error out at load_vk
        let vk_path = super::vk_path_for_version(data_dir, 2);
        fs::create_dir_all(vk_path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(&vk_path).unwrap();
        f.write_all(b"invalid vk").unwrap();
        drop(f);

        let root = fr_from_be_bytes(&[1u8; 32]).unwrap();
        let scope = fr_from_be_bytes(&[2u8; 32]).unwrap();
        let nullf = fr_from_be_bytes(&[3u8; 32]).unwrap();
        // create a default poseidon params Arc from config so we can call the verifier
        let poseidon_params = match load_poseidon_params(data_dir) {
            Ok(p) => std::sync::Arc::new(p),
            Err(_) => std::sync::Arc::new(ark_crypto_primitives::sponge::poseidon::PoseidonConfig::new(8, 57, 5, vec![], vec![], 2, 1)),
        };
        let err = verify_groth16_bn254(data_dir, 2, b"bad-proof", &[root, scope, nullf], &poseidon_params).err().unwrap();
        // The error may come from vk deserialization or proof deserialization; both are acceptable for this smoke path
        let s = format!("{}", err);
        assert!(s.contains("deserialize verifying key") || s.contains("deserialize groth16 proof"));
    }
}
