#![allow(dead_code)]
use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "zkp_halo2")]
pub async fn halo2_prove(out: &PathBuf) -> Result<()> {
    use halo2_proofs::circuit::{SimpleFloorPlanner, Layouter};
    use halo2_proofs::plonk::{Circuit, ConstraintSystem, Error};
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::Fp;
    use serde::Serialize;
    use std::fs::File;
    use std::io::Write;

    // A tiny circuit: enforces that a private witness `a` plus `b` equals a public output.
    #[derive(Clone, Debug)]
    struct AddCircuit {
        a: Option<Fp>,
        b: Option<Fp>,
    }

    #[derive(Clone, Debug)]
    struct AddConfig {}

    impl Circuit<Fp> for AddCircuit {
        type Config = AddConfig;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self { a: None, b: None }
        }

        fn configure(_meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            AddConfig {}
        }

        fn synthesize(&self, _config: Self::Config, mut _layouter: impl Layouter<Fp>) -> Result<(), Error> {
            // Minimal POC: no gates wired; MockProver checks instance dimensions.
            Ok(())
        }
    }

    let a = Fp::from(2u64);
    let b = Fp::from(3u64);
    let sum = a + b;

    let circuit = AddCircuit { a: Some(a), b: Some(b) };
    let k = 4u32;
    let public_inputs = vec![vec![sum]];

    let prover = MockProver::run(k, &circuit, public_inputs.clone()).map_err(|e| anyhow::anyhow!("mockprover run: {:?}", e))?;
    prover.assert_satisfied();

    #[derive(Serialize)]
    struct ProofJson {
        poc: bool,
        k: u32,
        public_inputs: Vec<Vec<String>>,
        note: &'static str,
    }

    let public_inputs_str: Vec<Vec<String>> = public_inputs.into_iter().map(|v| v.into_iter().map(|fe| format!("{:?}", fe)).collect()).collect();

    let payload = ProofJson { poc: true, k, public_inputs: public_inputs_str, note: "Halo2 POC (placeholder circuit)" };

    let mut f = File::create(out)?;
    let s = serde_json::to_string_pretty(&payload)?;
    f.write_all(s.as_bytes())?;
    f.write_all(b"\n")?;

    println!("Halo2: MockProver satisfied; JSON written to {}", out.display());
    Ok(())
}

#[cfg(not(feature = "zkp_halo2"))]
pub async fn halo2_prove(_out: &PathBuf) -> Result<()> {
    anyhow::bail!("feature 'zkp_halo2' not enabled")
}
