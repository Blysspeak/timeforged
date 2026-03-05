use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;

use timeforged_core::models::ReportRequest;

use crate::app::AppState;
use crate::service::{card_service, report_service, user_service};
use crate::storage::sqlite;

#[derive(Debug, Deserialize)]
pub struct CardQuery {
    pub key: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_theme() -> String {
    "dark".into()
}

fn default_days() -> u32 {
    365
}

/// Private card: GET /api/v1/card.svg?key=...
/// Always requires API key.
pub async fn card_svg(
    State(state): State<AppState>,
    Query(params): Query<CardQuery>,
) -> impl IntoResponse {
    let user = match params.key {
        Some(ref key) => match user_service::authenticate(&state.db, key).await {
            Ok(u) => u,
            Err(_) => return (StatusCode::UNAUTHORIZED, "invalid api key").into_response(),
        },
        None => return (StatusCode::UNAUTHORIZED, "api key required").into_response(),
    };

    render_card(&state, user.id, &params.theme, params.days).await
}

#[derive(Debug, Deserialize)]
pub struct PublicCardQuery {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_days")]
    pub days: u32,
}

/// Public card: GET /api/v1/card/:username.svg
/// Only works if user has public_profile enabled.
/// Returns 404 for both non-existent and private users (prevents enumeration).
pub async fn public_card_svg(
    State(state): State<AppState>,
    Path(username_svg): Path<String>,
    Query(params): Query<PublicCardQuery>,
) -> impl IntoResponse {
    let username = username_svg.strip_suffix(".svg").unwrap_or(&username_svg);

    let user = match sqlite::get_user_by_username(&state.db, username).await {
        Ok(Some(u)) if u.public_profile => u,
        // Same 404 for non-existent, private, or DB error — prevents user enumeration
        _ => return (StatusCode::NOT_FOUND, "not found").into_response(),
    };

    render_card(&state, user.id, &params.theme, params.days).await
}

async fn render_card(
    state: &AppState,
    user_id: uuid::Uuid,
    theme: &str,
    days: u32,
) -> axum::response::Response {
    let days = days.clamp(1, 365);
    let now = Utc::now();
    let from = now - chrono::Duration::days(days as i64);

    let req = ReportRequest {
        from: Some(from),
        to: Some(now),
        project: None,
        language: None,
    };

    let summary = match report_service::get_summary(
        &state.db,
        user_id,
        &req,
        state.config.idle_timeout,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get summary: {e}"),
            )
                .into_response()
        }
    };

    let theme = card_service::Theme::from_str(theme);
    let svg = card_service::render_svg(&summary, theme);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=1800"),
        ],
        svg,
    )
        .into_response()
}
