use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Erreur de signature: {0}")]
    SignatureError(String),
    
    #[error("Clé invalide: {0}")]
    InvalidKey(String),
    
    #[error("Erreur de vérification: {0}")]
    VerificationError(String),
    
    #[error("Erreur de sérialisation: {0}")]
    SerializationError(String),
    
    #[error("Erreur générique: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, CryptoError>;
