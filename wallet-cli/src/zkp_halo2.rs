#![allow(dead_code)]
use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "zkp_halo2")]
pub async fn halo2_prove(out: &PathBuf) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    // Minimal Halo2 POC circuit (membership + nullifier skeleton):
    // - private inputs: `leaf` (a) and `nullifier` (b)
    // - public input: `root` such that a + b = root (field arithmetic skeleton)
    // This is intentionally simple: it provides a scaffold for replacing the
    // adder with Poseidon/Merkle constraints later. We run MockProver to
    // validate the circuit and then write a small JSON 'proof' with public
    // inputs so the rest of the system can consume a stable artifact.

    use halo2_proofs::plonk::{Advice, Column, ConstraintSystem, Error, Instance, Selector};
    use halo2_proofs::{circuit::{Layouter, SimpleFloorPlanner, Value}, dev::MockProver, plonk::Circuit};
    use halo2_proofs::poly::Rotation;
    use halo2_proofs::pasta::Fp; // field for the simple POC
    use serde::Serialize;

    // Replace the simple arithmetic scaffold with a Poseidon-like MiMC rounds based
    // skeleton: compute a simple hash h = ((leaf + c0)^2 * (leaf + c0)) + sibling + c1
    // and constrain also a nullifier placeholder. The idea is to show how to wire
    // a small hash-like gadget and expose both h and the nullifier hash as public inputs.

    #[derive(Clone, Debug)]
    struct MembershipCircuit {
        leaf: Option<Fp>,
        sibling: Option<Fp>,
        nullifier: Option<Fp>,
    }

    #[derive(Clone, Debug)]
    struct Config {
        in0: Column<Advice>,
        in1: Column<Advice>,
        in2: Column<Advice>,

        t1_0: Column<Advice>,
        t2_0: Column<Advice>,
        t3_0: Column<Advice>,
        t4_0: Column<Advice>,

        t1_1: Column<Advice>,
        t2_1: Column<Advice>,
        t3_1: Column<Advice>,
        t4_1: Column<Advice>,

        t1_2: Column<Advice>,
        t2_2: Column<Advice>,
        t3_2: Column<Advice>,
        t4_2: Column<Advice>,

        h: Column<Advice>,
        null_hash: Column<Advice>,

        instance: Column<Instance>,
        s: Selector,
    }

    use halo2_proofs::plonk::Expression;

    impl Circuit<Fp> for MembershipCircuit {
        type Config = Config;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self { leaf: None, sibling: None, nullifier: None }
        }

        fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            let in0 = meta.advice_column();
            let in1 = meta.advice_column();
            let in2 = meta.advice_column();

            let t1_0 = meta.advice_column();
            let t2_0 = meta.advice_column();
            let t3_0 = meta.advice_column();
            let t4_0 = meta.advice_column();

            let t1_1 = meta.advice_column();
            let t2_1 = meta.advice_column();
            let t3_1 = meta.advice_column();
            let t4_1 = meta.advice_column();

            let t1_2 = meta.advice_column();
            let t2_2 = meta.advice_column();
            let t3_2 = meta.advice_column();
            let t4_2 = meta.advice_column();

            let h = meta.advice_column();
            let null_hash = meta.advice_column();

            let instance = meta.instance_column();
            let s = meta.selector();

            for col in [in0, in1, in2, t1_0, t2_0, t3_0, t4_0, t1_1, t2_1, t3_1, t4_1, t1_2, t2_2, t3_2, t4_2, h, null_hash] {
                meta.enable_equality(col);
            }

            // S-box gates for each element: compute x^5 via sequence of squares and multiply
            meta.create_gate("sbox_0_sq", |meta| {
                let s_q = meta.query_selector(s);
                let t2 = meta.query_advice(t2_0, Rotation::cur());
                let x = meta.query_advice(in0, Rotation::cur());
                vec![s_q * (t2 - x.clone() * x)]
            });
            meta.create_gate("sbox_0_4th", |meta| {
                let s_q = meta.query_selector(s);
                let t3 = meta.query_advice(t3_0, Rotation::cur());
                let t2 = meta.query_advice(t2_0, Rotation::cur());
                vec![s_q * (t3 - t2.clone() * t2)]
            });
            meta.create_gate("sbox_0_5th", |meta| {
                let s_q = meta.query_selector(s);
                let t4 = meta.query_advice(t4_0, Rotation::cur());
                let t3 = meta.query_advice(t3_0, Rotation::cur());
                let x = meta.query_advice(in0, Rotation::cur());
                vec![s_q * (t4 - t3.clone() * x)]
            });

            // element 1
            meta.create_gate("sbox_1_sq", |meta| {
                let s_q = meta.query_selector(s);
                let t2 = meta.query_advice(t2_1, Rotation::cur());
                let x = meta.query_advice(in1, Rotation::cur());
                vec![s_q * (t2 - x.clone() * x)]
            });
            meta.create_gate("sbox_1_4th", |meta| {
                let s_q = meta.query_selector(s);
                let t3 = meta.query_advice(t3_1, Rotation::cur());
                let t2 = meta.query_advice(t2_1, Rotation::cur());
                vec![s_q * (t3 - t2.clone() * t2)]
            });
            meta.create_gate("sbox_1_5th", |meta| {
                let s_q = meta.query_selector(s);
                let t4 = meta.query_advice(t4_1, Rotation::cur());
                let t3 = meta.query_advice(t3_1, Rotation::cur());
                let x = meta.query_advice(in1, Rotation::cur());
                vec![s_q * (t4 - t3.clone() * x)]
            });

            // element 2
            meta.create_gate("sbox_2_sq", |meta| {
                let s_q = meta.query_selector(s);
                let t2 = meta.query_advice(t2_2, Rotation::cur());
                let x = meta.query_advice(in2, Rotation::cur());
                vec![s_q * (t2 - x.clone() * x)]
            });
            meta.create_gate("sbox_2_4th", |meta| {
                let s_q = meta.query_selector(s);
                let t3 = meta.query_advice(t3_2, Rotation::cur());
                let t2 = meta.query_advice(t2_2, Rotation::cur());
                vec![s_q * (t3 - t2.clone() * t2)]
            });
            meta.create_gate("sbox_2_5th", |meta| {
                let s_q = meta.query_selector(s);
                let t4 = meta.query_advice(t4_2, Rotation::cur());
                let t3 = meta.query_advice(t3_2, Rotation::cur());
                let x = meta.query_advice(in2, Rotation::cur());
                vec![s_q * (t4 - t3.clone() * x)]
            });

            // Mixing gate: h = 2*t4_0 + 1*t4_1 + 1*t4_2 + rc0
            // and null_hash = t4_2 + rc1 (we'll set rc1 = 0 for now)
            meta.create_gate("mix", |meta| {
                let s_q = meta.query_selector(s);
                let t4_0_q = meta.query_advice(t4_0, Rotation::cur());
                let t4_1_q = meta.query_advice(t4_1, Rotation::cur());
                let t4_2_q = meta.query_advice(t4_2, Rotation::cur());
                let h_q = meta.query_advice(h, Rotation::cur());
                let null_q = meta.query_advice(null_hash, Rotation::cur());
                let inst0 = meta.query_instance(instance, Rotation::cur());
                let inst1 = meta.query_instance(instance, Rotation::next());

                // constants inline as Expressions
                let rc0 = Expression::Constant(Fp::from(9u64));
                let rc1 = Expression::Constant(Fp::from(0u64));

                vec![
                    s_q.clone() * (h_q - (t4_0_q.clone() * Expression::Constant(Fp::from(2u64)) + t4_1_q.clone() + t4_2_q.clone() + rc0 + inst0.clone() - inst0.clone())),
                    s_q * (null_q - (t4_2_q + rc1 + inst1.clone() - inst1.clone())),
                ]
            });

            Config {
                in0,
                in1,
                in2,
                t1_0,
                t2_0,
                t3_0,
                t4_0,
                t1_1,
                t2_1,
                t3_1,
                t4_1,
                t1_2,
                t2_2,
                t3_2,
                t4_2,
                h,
                null_hash,
                instance,
                s,
            }
        }

        fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
            // assign all values in a single region (single row)
            let (h_cell, null_cell) = layouter.assign_region(|| "poseidon_assign", |mut region| -> Result<_, Error> {
                let offset = 0;

                let in0_val = self.leaf.unwrap_or(Fp::from(0u64));
                let in1_val = self.sibling.unwrap_or(Fp::from(0u64));
                let in2_val = self.nullifier.unwrap_or(Fp::from(0u64));

                // S-box computations x^5 for each input
                let t2_0_val = in0_val * in0_val;
                let t3_0_val = t2_0_val * t2_0_val;
                let t4_0_val = t3_0_val * in0_val; // x^5

                let t2_1_val = in1_val * in1_val;
                let t3_1_val = t2_1_val * t2_1_val;
                let t4_1_val = t3_1_val * in1_val;

                let t2_2_val = in2_val * in2_val;
                let t3_2_val = t2_2_val * t2_2_val;
                let t4_2_val = t3_2_val * in2_val;

                // mixing: h = 2*t4_0 + t4_1 + t4_2 + rc0
                let rc0 = Fp::from(9u64);
                let h_val = t4_0_val + t4_0_val + t4_1_val + t4_2_val + rc0;
                let null_hash_val = t4_2_val; // placeholder

                region.assign_advice(|| "in0", config.in0, offset, || Value::known(in0_val))?;
                region.assign_advice(|| "in1", config.in1, offset, || Value::known(in1_val))?;
                region.assign_advice(|| "in2", config.in2, offset, || Value::known(in2_val))?;

                region.assign_advice(|| "t2_0", config.t2_0, offset, || Value::known(t2_0_val))?;
                region.assign_advice(|| "t3_0", config.t3_0, offset, || Value::known(t3_0_val))?;
                region.assign_advice(|| "t4_0", config.t4_0, offset, || Value::known(t4_0_val))?;

                region.assign_advice(|| "t2_1", config.t2_1, offset, || Value::known(t2_1_val))?;
                region.assign_advice(|| "t3_1", config.t3_1, offset, || Value::known(t3_1_val))?;
                region.assign_advice(|| "t4_1", config.t4_1, offset, || Value::known(t4_1_val))?;

                region.assign_advice(|| "t2_2", config.t2_2, offset, || Value::known(t2_2_val))?;
                region.assign_advice(|| "t3_2", config.t3_2, offset, || Value::known(t3_2_val))?;
                region.assign_advice(|| "t4_2", config.t4_2, offset, || Value::known(t4_2_val))?;

                let h_cell = region.assign_advice(|| "h", config.h, offset, || Value::known(h_val))?;
                let null_cell = region.assign_advice(|| "null_hash", config.null_hash, offset, || Value::known(null_hash_val))?;

                config.s.enable(&mut region, offset)?;

                Ok((h_cell.cell(), null_cell.cell()))
            })?;

            layouter.constrain_instance(h_cell, config.instance, 0)?;
            layouter.constrain_instance(null_cell, config.instance, 1)?;

            Ok(())
        }
    }

        // choose test values and run MockProver (must match the in-circuit assignment logic)
        let leaf = Fp::from(5u64);
        let sibling = Fp::from(11u64);
        let nullifier = Fp::from(13u64);

        // compute x^5 for each input (same as synthesize)
        let t2_0_val = leaf * leaf;
        let t3_0_val = t2_0_val * t2_0_val;
        let t4_0_val = t3_0_val * leaf;

        let t2_1_val = sibling * sibling;
        let t3_1_val = t2_1_val * t2_1_val;
        let t4_1_val = t3_1_val * sibling;

        let t2_2_val = nullifier * nullifier;
        let t3_2_val = t2_2_val * t2_2_val;
        let t4_2_val = t3_2_val * nullifier;

        let rc0 = Fp::from(9u64);
        let h = t4_0_val + t4_0_val + t4_1_val + t4_2_val + rc0;
        let null_hash = t4_2_val;

        let circuit = MembershipCircuit { leaf: Some(leaf), sibling: Some(sibling), nullifier: Some(nullifier) };
        let k = 4u32;
        let public_inputs = vec![vec![h], vec![null_hash]];

    println!("Running MockProver (Merkle+nullifier skeleton) k={}...", k);
    let prover = MockProver::run(k, &circuit, public_inputs.clone())?;
    prover.assert_satisfied();

    #[derive(Serialize)]
    struct ProofJson {
        poc: bool,
        k: u32,
        public_inputs: Vec<Vec<String>>,
        note: &'static str,
    }

    let public_inputs_str: Vec<Vec<String>> = public_inputs.into_iter().map(|v| v.into_iter().map(|fe| format!("{:?}", fe)).collect()).collect();

    let payload = ProofJson {
        poc: true,
        k,
        public_inputs: public_inputs_str,
        note: "MockProver merkle+nullifier skeleton (replace MiMC-style rounds with real Poseidon gadget)",
    };

    let mut f = File::create(out)?;
    let s = serde_json::to_string_pretty(&payload)?;
    f.write_all(s.as_bytes())?;
    f.write_all(b"\n")?;

    println!("Halo2 POC: MockProver satisfied; JSON written to {}", out.display());
    Ok(())
}

#[cfg(not(feature = "zkp_halo2"))]
pub async fn halo2_prove(_out: &PathBuf) -> Result<()> {
    anyhow::bail!("feature 'zkp_halo2' not enabled")
}
