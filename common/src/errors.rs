use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("Erreur cryptographique: {0}")]
    CryptoError(#[from] crypto_lib::CryptoError),
    
    #[error("Bloc invalide: {0}")]
    InvalidBlock(String),
    
    #[error("Transaction invalide: {0}")]
    InvalidTransaction(String),
    
    #[error("Compte non trouvé: {0}")]
    AccountNotFound(String),
    
    #[error("Droits insuffisants: {0}")]
    InsufficientPermissions(String),
    
    #[error("Loi non trouvée: {0}")]
    LawNotFound(String),
    
    #[error("Vote invalide: {0}")]
    InvalidVote(String),
    
    #[error("Erreur de sérialisation: {0}")]
    SerializationError(String),
    
    #[error("Erreur générique: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, BlockchainError>;
