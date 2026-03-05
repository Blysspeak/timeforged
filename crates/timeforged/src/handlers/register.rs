use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use timeforged_core::api::{ErrorResponse, RegisterRequest, RegisterResponse};

use crate::app::AppState;
use crate::service::user_service;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    if req.username.is_empty() || req.username.len() > 32 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "username must be 1-32 characters".into(),
            }),
        )
            .into_response();
    }

    // Only allow alphanumeric, hyphens, underscores
    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "username must be alphanumeric (hyphens and underscores allowed)".into(),
            }),
        )
            .into_response();
    }

    let user = match user_service::create_user(
        &state.db,
        &req.username,
        req.display_name.as_deref(),
    )
    .await
    {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: "username already taken".into(),
                }),
            )
                .into_response()
        }
    };

    let raw_key = user_service::generate_api_key();
    let hash = user_service::hash_api_key(&raw_key);

    if let Err(e) = crate::storage::sqlite::create_api_key(&state.db, user.id, &hash, "default")
        .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("failed to create api key: {e}"),
            }),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(RegisterResponse {
            username: user.username,
            api_key: raw_key,
        }),
    )
        .into_response()
}
