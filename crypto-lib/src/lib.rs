//! crypto-lib — cryptographic primitives used in the workspace.
//!
//! This crate exposes small types and utilities for:
//! - Ed25519 key pairs (`KeyPair`, `PublicKey`, `PrivateKey`),
//! - signatures (`Signature`),
//! - hashing (SHA-256) (`Hash`),
//! - error handling (`CryptoError`).
//!
//! Quick example:
//!
//! ```rust
//! use crypto_lib::{KeyPair, Signature};
//!
//! // Generate a key pair and sign a message
//! let kp = KeyPair::generate();
//! let sig: Signature = kp.sign(b"hello");
//! kp.verify(b"hello", &sig).expect("verification");
//! ```

#![deny(missing_docs)]

/// Key pair utilities: public/private key types and helpers.
pub mod keypair;

/// Signature type and helpers for Ed25519 signatures.
pub mod signature;

/// SHA-256 hash utilities and value type.
pub mod hash;

/// Error types and result alias used across the crate.
pub mod errors;

pub use keypair::{KeyPair, PublicKey, PrivateKey};
pub use signature::Signature;
pub use hash::Hash;
pub use errors::CryptoError;
