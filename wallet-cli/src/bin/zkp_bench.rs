use std::time::Instant;
use std::env;
use anyhow::Result;

fn usage_and_exit() -> ! {
    eprintln!("Usage: zkp_bench <data_dir> <vk_version> <root_hex> <scope> <nullifier_hex> <leaf_hex> <secret_hex> <directions> [iterations]");
    std::process::exit(2);
}

// Include the local zkp.rs source directly into a module so the bench can call
// the prove function. Using `include!` with CARGO_MANIFEST_DIR avoids relative
// path resolution issues for src/bin files.
#[cfg(feature = "zkp_groth16")]
mod local_zkp {
    #[allow(unused)]
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/zkp.rs"));
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 9 {
        usage_and_exit();
    }
    let data_dir = &args[1];
    let vk_version: u32 = args[2].parse().expect("vk_version must be integer");
    let root_hex = &args[3];
    let scope = &args[4];
    let nullifier_hex = &args[5];
    let leaf_hex = &args[6];
    let secret_hex = &args[7];
    let directions = &args[8];
    let iterations: usize = if args.len() > 9 { args[9].parse().unwrap_or(1) } else { 1 };

    println!("zkp_bench: data_dir={} vk_version={} iterations={}", data_dir, vk_version, iterations);

    #[cfg(not(feature = "zkp_groth16"))]
    {
        eprintln!("This benchmark requires the feature 'zkp_groth16'. Build with --features zkp_groth16");
        std::process::exit(2);
    }

    #[cfg(feature = "zkp_groth16")]
    {
    use local_zkp::prove_membership_nullifier;

        // warm up one run
        let start = Instant::now();
        let (proof, _pub_inputs) = prove_membership_nullifier(
            data_dir,
            vk_version,
            root_hex,
            scope,
            nullifier_hex,
            leaf_hex,
            secret_hex,
            &[], // caller can extend to pass siblings
            directions,
        )?;
        let dur = start.elapsed();
        println!("Warmup run duration: {:?}, proof bytes: {}", dur, proof.len());

        // timed iterations
        let mut total = std::time::Duration::ZERO;
        for i in 0..iterations {
            let t0 = Instant::now();
            let (proof, _pub_inputs) = prove_membership_nullifier(
                data_dir,
                vk_version,
                root_hex,
                scope,
                nullifier_hex,
                leaf_hex,
                secret_hex,
                &[],
                directions,
            )?;
            let dt = t0.elapsed();
            total += dt;
            println!("iter {}: {:?} (proof {} bytes)", i, dt, proof.len());
        }
        let avg = total / (iterations as u32);
        println!("Average prove duration over {} iterations: {:?}", iterations, avg);
    }

    Ok(())
}
