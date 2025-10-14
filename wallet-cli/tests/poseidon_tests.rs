#[cfg(test)]
mod tests {
    use halo2_gadgets_poseidon::PoseidonParams;
    use halo2_proofs::pasta::Fp;
    use halo2_proofs::dev::MockProver;
    use wallet_cli::zkp_halo2::halo2_prove;
    use std::path::PathBuf;

    #[test]
    fn test_poseidon_params_generation() {
        let params = PoseidonParams::new(3, 8, 57, None);
        assert_eq!(params.width, 3);
        assert_eq!(params.full_rounds, 8);
        assert_eq!(params.partial_rounds, 57);
    }

    #[tokio::test]
    async fn test_halo2_prove() {
        let out_path = PathBuf::from("test_halo2_proof.json");
        let result = halo2_prove(&out_path).await;
        assert!(result.is_ok(), "Halo2 proof generation failed: {:?}", result);

        // Verify the output file exists
        assert!(out_path.exists(), "Proof output file was not created");

        // Clean up
        std::fs::remove_file(out_path).expect("Failed to clean up test output file");
    }

    #[test]
    fn test_poseidon_native_vs_circuit() {
        use halo2_gadgets_poseidon::{PoseidonChip, PoseidonConfig};
        use halo2_proofs::circuit::{Layouter, SimpleFloorPlanner};
        use halo2_proofs::plonk::{Circuit, ConstraintSystem, Error};

        // Native Poseidon hash
        let params = PoseidonParams::new(3, 8, 57, None);
        let native_hash = params.hash(&[Fp::from(5), Fp::from(11), Fp::from(13)]);

        // Circuit Poseidon hash
        #[derive(Clone, Debug)]
        struct PoseidonCircuit {
            leaf: Option<Fp>,
            sibling: Option<Fp>,
            nullifier: Option<Fp>,
        }

        impl Circuit<Fp> for PoseidonCircuit {
            type Config = PoseidonConfig;
            type FloorPlanner = SimpleFloorPlanner;

            fn without_witnesses(&self) -> Self {
                Self { leaf: None, sibling: None, nullifier: None }
            }

            fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
                PoseidonChip::configure(meta, params.clone())
            }

            fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
                let poseidon_chip = PoseidonChip::construct(config);

                let leaf = self.leaf.ok_or(Error::Synthesis)?;
                let sibling = self.sibling.ok_or(Error::Synthesis)?;
                let nullifier = self.nullifier.ok_or(Error::Synthesis)?;

                let hash = poseidon_chip.hash(layouter.namespace(|| "Poseidon hash"), &[leaf, sibling, nullifier])?;

                layouter.constrain_instance(hash.cell(), config.instance, 0)?;

                Ok(())
            }
        }

        let circuit = PoseidonCircuit {
            leaf: Some(Fp::from(5)),
            sibling: Some(Fp::from(11)),
            nullifier: Some(Fp::from(13)),
        };

        let k = 4u32;
        let public_inputs = vec![vec![native_hash]];

        let prover = MockProver::run(k, &circuit, public_inputs.clone()).unwrap();
        prover.assert_satisfied();
    }
}