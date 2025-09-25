use axum::{extract::{State, Path}, response::Json, http::StatusCode};
use serde_json;
use uuid::Uuid;
use crypto_lib::{PublicKey, Signature};
use super::super::{AppState, types::*};

/// Simplified handler to check account identity status
pub async fn identity_status_simple_handler(
    State(node): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Ok(acc_id) = Uuid::parse_str(&account_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let blockchain = node.blockchain.read().await;
    let account = blockchain.accounts.get(&acc_id);

    match account {
        Some(acc) if acc.metadata.verified => {
            Ok(Json(serde_json::json!({
                "status": "verified",
                "account_id": acc_id,
                "verified": true,
                "verification_date": acc.created_at.to_rfc3339()
            })))
        }
        Some(_acc) => {
            Ok(Json(serde_json::json!({
                "status": "unverified", 
                "account_id": acc_id,
                "verified": false
            })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Handler to verify a cryptographic signature
pub async fn crypto_verify_handler(
    Json(request): Json<CryptoVerifyRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    use sha2::Digest;
    
    // Parse public key
    let public_key = match PublicKey::from_hex(&request.public_key) {
        Ok(pk) => pk,
        Err(_) => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_public_key".into(),
                message: "Invalid public key".into(),
            })
        )),
    };

    // Decode message (try hex then raw)
    let message_bytes = if let Ok(bytes) = hex::decode(&request.message) {
        bytes
    } else {
        request.message.as_bytes().to_vec()
    };

    // Decode signature (try hex)
    let signature_bytes = match hex::decode(&request.signature) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_signature_encoding".into(),
                    message: "Unsupported signature encoding".into(),
                })
            ));
        }
    };

    let signature = match Signature::from_bytes(&signature_bytes) {
        Ok(sig) => sig,
        Err(_) => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid_signature_format".into(),
                message: "Invalid signature format".into(),
            })
        )),
    };

    // Verify signature
    let is_valid = public_key.verify(&message_bytes, &signature).is_ok();

    Ok(Json(serde_json::json!({
        "valid": is_valid,
        "public_key": request.public_key,
        "message_hash": hex::encode(sha2::Sha256::digest(&message_bytes)),
        "verified_at": chrono::Utc::now().to_rfc3339()
    })))
}

/// Handler to check voting eligibility for an account
pub async fn vote_eligibility_simple_handler(
    State(node): State<AppState>,
    Path(account_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Ok(acc_id) = Uuid::parse_str(&account_id) else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let blockchain = node.blockchain.read().await;
    
    let eligibility = if let Some(account) = blockchain.accounts.get(&acc_id) {
        serde_json::json!({
            "account_id": acc_id,
            "can_vote": account.metadata.verified && account.is_active,
            "verified": account.metadata.verified,
            "active": account.is_active,
            "reasons": if !account.metadata.verified {
                vec!["Account not verified"]
            } else if !account.is_active {
                vec!["Account inactive"]
            } else {
                vec![]
            }
        })
    } else {
        serde_json::json!({
            "account_id": acc_id,
            "can_vote": false,
            "verified": false,
            "active": false,
            "reasons": vec!["Account not found"]
        })
    };

    Ok(Json(eligibility))
}