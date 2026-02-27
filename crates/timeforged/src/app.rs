use axum::{Router, middleware, routing::{delete, get, post}};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use timeforged_core::config::AppConfig;

use crate::auth;
use crate::handlers::{events, health, reports, users};
use crate::web;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let authed = Router::new()
        // Events
        .route("/api/v1/events", post(events::create_event))
        .route("/api/v1/events/batch", post(events::create_batch))
        // Reports
        .route("/api/v1/reports/summary", get(reports::summary))
        .route("/api/v1/reports/sessions", get(reports::sessions))
        .route("/api/v1/reports/activity", get(reports::activity))
        // Users
        .route("/api/v1/me", get(users::me))
        .route("/api/v1/api-keys", post(users::create_api_key).get(users::list_api_keys))
        .route("/api/v1/api-keys/{id}", delete(users::delete_api_key))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let public = Router::new()
        .route("/health", get(health::health))
        .route("/api/v1/status", get(health::status));

    Router::new()
        .merge(authed)
        .merge(public)
        .fallback(web::static_handler)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
