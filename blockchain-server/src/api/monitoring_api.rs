use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::monitoring::{SecurityMonitor, SecurityEvent, MonitoringMetrics};
use crate::revocation::{RevocationList, RevocationReason};

/// Monitoring API state
pub struct MonitoringState {
    pub monitor: Arc<SecurityMonitor>,
    pub revocation: Arc<RevocationList>,
}

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    severity: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    #[serde(default = "default_hours")]
    hours: i64,
}

fn default_hours() -> i64 {
    24
}

/// GET /api/monitoring/metrics - Get current monitoring metrics
pub async fn get_metrics(
    State(state): State<Arc<MonitoringState>>,
) -> Result<Json<MonitoringMetrics>, StatusCode> {
    let metrics = state.monitor.get_metrics().await;
    Ok(Json(metrics))
}

/// GET /api/monitoring/events - Get recent security events
pub async fn get_events(
    State(state): State<Arc<MonitoringState>>,
    Query(query): Query<EventQuery>,
) -> impl IntoResponse {
    let events = if let Some(severity) = query.severity {
        state.monitor.get_events_by_severity(&severity, query.limit).await
    } else {
        state.monitor.get_recent_events(query.limit).await
    };

    match events {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// GET /api/monitoring/report - Generate security report
pub async fn generate_report(
    State(state): State<Arc<MonitoringState>>,
    Query(query): Query<ReportQuery>,
) -> impl IntoResponse {
    match state.monitor.generate_report(query.hours).await {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// POST /api/monitoring/reset - Reset metrics (admin only)
pub async fn reset_metrics(
    State(state): State<Arc<MonitoringState>>,
) -> impl IntoResponse {
    state.monitor.reset_metrics().await;
    (StatusCode::OK, Json(serde_json::json!({ "status": "metrics_reset" })))
}

// Revocation endpoints

#[derive(Debug, Deserialize)]
pub struct RevocationRequest {
    pub identifier: String,
    pub identifier_type: String,
    pub reason: String,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevocationCheckRequest {
    pub identifier: String,
    pub identifier_type: String,
}

#[derive(Debug, Serialize)]
pub struct RevocationCheckResponse {
    pub identifier: String,
    pub is_revoked: bool,
    pub details: Option<serde_json::Value>,
}

/// POST /api/revocation/revoke - Revoke an identifier (admin only)
pub async fn revoke_identifier(
    State(state): State<Arc<MonitoringState>>,
    Json(req): Json<RevocationRequest>,
) -> impl IntoResponse {
    let reason = RevocationReason::from_str(&req.reason)
        .unwrap_or(RevocationReason::Administrative);
    
    match state.revocation.revoke(
        &req.identifier,
        &req.identifier_type,
        reason,
        req.details.as_deref(),
    ).await {
        Ok(_) => {
            // Log the revocation event
            let _ = state.monitor.log_event(SecurityEvent::RevocationDetected {
                identifier: req.identifier.clone(),
                identifier_type: req.identifier_type.clone(),
            }).await;
            
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "revoked",
                    "identifier": req.identifier
                })),
            ).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// GET /api/revocation/check - Check if an identifier is revoked
pub async fn check_revocation(
    State(state): State<Arc<MonitoringState>>,
    Query(req): Query<RevocationCheckRequest>,
) -> impl IntoResponse {
    match state.revocation.is_revoked(&req.identifier, &req.identifier_type).await {
        Ok(is_revoked) => {
            let details = if is_revoked {
                state.revocation
                    .get_revocation(&req.identifier, &req.identifier_type)
                    .await
                    .ok()
                    .flatten()
                    .map(|e| serde_json::to_value(e).unwrap_or(serde_json::json!({})))
            } else {
                None
            };
            
            (
                StatusCode::OK,
                Json(RevocationCheckResponse {
                    identifier: req.identifier,
                    is_revoked,
                    details,
                }),
            ).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// GET /api/revocation/statistics - Get revocation statistics
pub async fn revocation_statistics(
    State(state): State<Arc<MonitoringState>>,
) -> impl IntoResponse {
    match state.revocation.get_statistics().await {
        Ok(stats) => (StatusCode::OK, Json(stats)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct RevocationListQuery {
    #[serde(default = "default_page_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_page_limit() -> i64 {
    100
}

/// GET /api/revocation/list - List all revocations (paginated)
pub async fn list_revocations(
    State(state): State<Arc<MonitoringState>>,
    Query(query): Query<RevocationListQuery>,
) -> impl IntoResponse {
    match state.revocation.list_revocations(query.limit, query.offset).await {
        Ok(list) => (StatusCode::OK, Json(list)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
