use axum::{Extension, Json, extract::{Query, State}, http::StatusCode, response::IntoResponse};

use timeforged_core::api::ErrorResponse;
use timeforged_core::models::ReportRequest;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::service::report_service;

pub async fn summary(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Query(req): Query<ReportRequest>,
) -> impl IntoResponse {
    match report_service::get_summary(&state.db, user.id, &req, state.config.idle_timeout).await {
        Ok(s) => (StatusCode::OK, Json(s)).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn sessions(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Query(req): Query<ReportRequest>,
) -> impl IntoResponse {
    match report_service::get_sessions(&state.db, user.id, &req, state.config.idle_timeout).await {
        Ok(s) => (StatusCode::OK, Json(s)).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn activity(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Query(req): Query<ReportRequest>,
) -> impl IntoResponse {
    match report_service::get_hourly_activity(&state.db, user.id, &req, state.config.idle_timeout)
        .await
    {
        Ok(s) => (StatusCode::OK, Json(s)).into_response(),
        Err(e) => error_response(e),
    }
}

fn error_response(e: timeforged_core::error::AppError) -> axum::response::Response {
    use timeforged_core::error::AppError;
    let (status, msg) = match &e {
        AppError::Validation(m) | AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
        AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".into()),
        AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
        AppError::Database(m) | AppError::Internal(m) => {
            (StatusCode::INTERNAL_SERVER_ERROR, m.clone())
        }
    };
    (status, Json(ErrorResponse { error: msg })).into_response()
}
