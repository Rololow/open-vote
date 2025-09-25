//! Module identité (Phase 2) – façade ré-exportant les sous-modules internes.
//! Objectifs Partie A:
//!  - Séparer DID / VC.
//!  - Fournir canonisation JSON déterministe + hash d'engagement.
//!  - Conserver une impl minimale de vérification (stub) avant résolution DID réelle.

#[cfg(feature = "identity")]
pub mod did;
#[cfg(feature = "identity")]
pub mod vc;
#[cfg(feature = "identity")]
pub mod commitment;

#[cfg(feature = "identity")]
pub use did::*;
#[cfg(feature = "identity")]
pub use vc::*;
#[cfg(feature = "identity")]
pub use commitment::*;

