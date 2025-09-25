use axum::{extract::State, response::Json, http::StatusCode};
use serde_json;
use tracing::{info, error};
use uuid::Uuid;
use common::{Transaction, TransactionType, Account};
use crypto_lib::{PublicKey, Signature};
use super::super::{AppState, types::*};

/// Handler pour créer un compte (signé par l'utilisateur)
pub async fn create_account_handler(
    State(node): State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Account creation request for pk: {}", request.public_key);

    // 1) Parse public key and signature
    let user_pk = match PublicKey::from_hex(&request.public_key) {
        Ok(pk) => pk,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { 
                    error: "invalid_public_key".into(), 
                    message: "Invalid public key".into() 
                })
            ));
        }
    };
    let sig = match Signature::from_hex(&request.signature) {
        Ok(s) => s,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { 
                    error: "invalid_signature".into(), 
                    message: "Invalid signature".into() 
                })
            ));
        }
    };

    // 2) Verify that the signature proves possession of the key
    // Canonical message format: "CREATE_ACCOUNT:<pubkey_hex>"
    let msg = format!("CREATE_ACCOUNT:{}", request.public_key);
    if let Err(e) = user_pk.verify(msg.as_bytes(), &sig) {
        error!("Account creation signature verification failed: {:?}", e);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse { 
                error: "unauthorized".into(), 
                message: "Invalid account creation signature".into() 
            })
        ));
    }

    // 3) Check for duplication by public key
    {
        let chain = node.blockchain.read().await;
        let exists = chain.accounts.values().any(|a| a.public_key == user_pk);
        if exists {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse { 
                    error: "account_exists".into(), 
                    message: "Account with this key already exists".into() 
                })
            ));
        }
    }

    // 4) Build Account object and on-chain transaction
    let mut account = Account::new(user_pk.clone());
    account.metadata.display_name = request.display_name.clone();
    account.metadata.bio = request.bio.clone();

    let sender_pk = node.public_key().clone();
    let mut tx = Transaction::new(
        TransactionType::CreateAccount(account.clone()),
        sender_pk,
        node.keypair.sign(b"temp"),
        0,
        0,
    );
    let sign_msg = format!(
        "TRANSACTION:{}:{}:{}",
        tx.id,
        tx.timestamp,
        tx.data_hash.to_hex()
    );
    tx.signature = node.keypair.sign(sign_msg.as_bytes());

    if let Err(e) = node.submit_transaction(tx).await {
        error!("Error submitting account creation transaction: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { 
                error: "tx_error".into(), 
                message: "Unable to add transaction".into() 
            })
        ));
    }

    // Mine for immediate visibility (optional)
    if let Err(e) = node.mine_block().await {
        error!("Error mining after account creation: {}", e);
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Account created successfully",
        "account": {
            "id": account.id,
            "public_key": account.public_key.to_hex(),
            "display_name": account.metadata.display_name,
            "bio": account.metadata.bio,
            "created_at": account.created_at.to_rfc3339(),
        }
    })))
}