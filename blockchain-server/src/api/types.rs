use serde::{Deserialize, Serialize};
use common::Transaction;

/// Structure pour la diffusion d'une transaction signée
#[derive(Deserialize)]
pub struct BroadcastTxRequest {
    /// La transaction complète, signée par le client
    pub transaction: Transaction,
}

/// Structure pour les réponses d'erreur
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

/// Structure pour supporter une proposition
#[derive(Deserialize)]
pub struct SupportProposalRequest { 
    pub supporter_public_key: String,
    pub signature: String,
}

/// Structure pour créer un compte
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CreateAccountRequest {
    pub public_key: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub signature: String, // Signature prouvant la possession de la clé privée
}

/// Structure pour soumettre une transaction
#[derive(Deserialize)]
pub struct SubmitTransactionRequest {
    pub transaction_type: String,
    pub from_account: Option<String>,
    pub to_account: Option<String>,
    pub amount: Option<f64>,
}

/// Structure pour créer une loi
#[derive(Deserialize)]
pub struct CreateLawRequest {
    pub title: String,
    pub description: String,
    pub category: String,
}

/// Structure pour créer une proposition citoyenne (proposition de loi)
#[derive(Deserialize)]
pub struct CreateProposalRequest {
    pub title: String,
    pub category: String,
    pub description: String,
    // Champs optionnels supplémentaires
    pub full_text: Option<String>,
    pub estimated_budget: Option<u64>,
    pub implementation_timeline: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author_id: Option<String>,
    pub author_name: Option<String>,
}

/// Structure pour soumettre un vote
#[derive(Deserialize)]
pub struct SubmitVoteRequest {
    pub law_id: String,
    pub vote_type: String, // "yes", "no", "abstain"
    pub voter_id: String,
}

/// Structure pour ajouter un pair
#[derive(Deserialize)]
pub struct AddPeerRequest { 
    pub address: String 
}

/// Structure pour les paramètres de pagination
#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

/// Structure pour vérifier une signature
#[derive(Deserialize)]
pub struct CryptoVerifyRequest {
    pub public_key: String,
    pub message: String, // hex or base64
    pub signature: String, // hex or base64
}