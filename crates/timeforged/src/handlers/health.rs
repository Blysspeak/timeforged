use axum::{Json, extract::State};

use timeforged_core::api::{HealthResponse, StatusResponse};

use crate::app::AppState;
use crate::storage::sqlite;

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        version: timeforged_core::VERSION.into(),
    })
}

pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let user_count = sqlite::count_users(&state.db).await.unwrap_or(0);
    let event_count = sqlite::count_events(&state.db).await.unwrap_or(0);

    Json(StatusResponse {
        status: "ok".into(),
        version: timeforged_core::VERSION.into(),
        user_count,
        event_count,
    })
}
