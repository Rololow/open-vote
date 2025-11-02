//! Library facade for `wallet-cli` so integration tests can import modules.
//!
//! This file re-exports the internal modules implemented for the binary.

pub mod identity;
pub mod keystore;

#[cfg(any(feature = "zkp_groth16", feature = "zkp_halo2"))]
pub mod zkp;

#[cfg(feature = "zkp_halo2")]
pub mod zkp_halo2;

// Small helper re-exports can be added here if tests expect them at crate root.
