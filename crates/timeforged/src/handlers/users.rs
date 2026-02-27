use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use timeforged_core::api::{CreateApiKeyRequest, ErrorResponse};
use timeforged_core::models::User;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::service::user_service;

pub async fn me(Extension(AuthUser(user)): Extension<AuthUser>) -> Json<User> {
    Json(user)
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Json(req): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    match user_service::create_api_key(&state.db, user.id, req).await {
        Ok(resp) => (StatusCode::CREATED, Json(resp)).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
) -> impl IntoResponse {
    match user_service::list_api_keys(&state.db, user.id).await {
        Ok(keys) => Json(keys).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    Extension(AuthUser(user)): Extension<AuthUser>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let key_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid key id".into(),
                }),
            )
                .into_response()
        }
    };

    match user_service::delete_api_key(&state.db, user.id, key_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
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
