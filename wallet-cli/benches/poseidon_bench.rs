#![cfg(feature = "zkp_halo2")]

use criterion::{criterion_group, criterion_main, Criterion};
use halo2_gadgets_poseidon::PoseidonParams;
use halo2_proofs::dev::MockProver;
use halo2_proofs::plonk::{Circuit, ConstraintSystem, Error};
use halo2_proofs::pasta::Fp;
use halo2_poseidon::{PoseidonChip, PoseidonConfig};

#[derive(Clone, Debug)]
struct PoseidonCircuit {
    leaf: Option<Fp>,
    sibling: Option<Fp>,
    nullifier: Option<Fp>,
}

impl Circuit<Fp> for PoseidonCircuit {
    type Config = PoseidonConfig;
    type FloorPlanner = halo2_proofs::circuit::SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self { leaf: None, sibling: None, nullifier: None }
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        PoseidonChip::configure(meta, PoseidonParams::new(3, 8, 57, None))
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl halo2_proofs::circuit::Layouter<Fp>) -> Result<(), Error> {
        let poseidon_chip = PoseidonChip::construct(config);

        let leaf = self.leaf.ok_or(Error::Synthesis)?;
        let sibling = self.sibling.ok_or(Error::Synthesis)?;
        let nullifier = self.nullifier.ok_or(Error::Synthesis)?;

        let _hash = poseidon_chip.hash(layouter.namespace(|| "Poseidon hash"), &[leaf, sibling, nullifier])?;

        Ok(())
    }
}

fn bench_poseidon(c: &mut Criterion) {
    let params = PoseidonParams::new(3, 8, 57, None);

    c.bench_function("poseidon_hash", |b| {
        b.iter(|| {
            let _ = params.hash(&[Fp::from(5), Fp::from(11), Fp::from(13)]);
        });
    });
}

fn bench_mock_prover(c: &mut Criterion) {
    let circuit = PoseidonCircuit {
        leaf: Some(Fp::from(5)),
        sibling: Some(Fp::from(11)),
        nullifier: Some(Fp::from(13)),
    };

    c.bench_function("mock_prover_poseidon", |b| {
        b.iter(|| {
            let k = 4u32;
            let public_inputs = vec![vec![Fp::from(5) + Fp::from(11) + Fp::from(13)]];
            let prover = MockProver::run(k, &circuit, public_inputs.clone()).unwrap();
            prover.assert_satisfied();
        });
    });
}

criterion_group!(benches, bench_poseidon, bench_mock_prover);
criterion_main!(benches);