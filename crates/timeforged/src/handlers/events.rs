use axum::{Extension, Json, extract::State, http::StatusCode, response::IntoResponse};

use timeforged_core::api::{BatchEventRequest, CreateEventRequest, ErrorResponse};

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::service::event_service;

pub async fn create_event(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Json(req): Json<CreateEventRequest>,
) -> impl IntoResponse {
    match event_service::create_event(&state.db, user.id, req).await {
        Ok(resp) => (StatusCode::CREATED, Json(resp)).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn create_batch(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Json(req): Json<BatchEventRequest>,
) -> impl IntoResponse {
    match event_service::create_batch(&state.db, user.id, req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
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
