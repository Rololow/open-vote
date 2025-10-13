use thiserror::Error;

/// Common error type for `crypto-lib` operations.
///
/// Each variant represents a class of errors returned by the crate's
/// public APIs.
#[derive(Error, Debug)]
pub enum CryptoError {
    /// An error occurred during signature creation or processing.
    #[error("Signature error: {0}")]
    SignatureError(String),

    /// The provided key material is invalid or malformed.
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// A signature verification failed.
    #[error("Verification error: {0}")]
    VerificationError(String),

    /// Error while serializing/deserializing values.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Generic catch-all error.
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Result type returned by `crypto-lib` operations.
pub type Result<T> = std::result::Result<T, CryptoError>;
