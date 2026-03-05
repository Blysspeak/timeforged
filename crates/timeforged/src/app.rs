use axum::{Router, middleware, routing::{delete, get, post, put}};
use axum::http::{HeaderValue, Method};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use timeforged_core::config::AppConfig;

use crate::auth;
use crate::handlers::{card, events, health, register, reports, users, watcher};
use crate::rate_limit;
use crate::watcher::WatcherCommand;
use crate::web;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
    pub watcher_tx: mpsc::Sender<WatcherCommand>,
}

pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin([
            "https://blysspeak.space".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:6175".parse::<HeaderValue>().unwrap(),
            "http://localhost:6175".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::HeaderName::from_static("x-api-key"),
        ]);

    let authed = Router::new()
        // Events
        .route("/api/v1/events", post(events::create_event).get(events::list_events))
        .route("/api/v1/events/batch", post(events::create_batch))
        // Reports
        .route("/api/v1/reports/summary", get(reports::summary))
        .route("/api/v1/reports/sessions", get(reports::sessions))
        .route("/api/v1/reports/activity", get(reports::activity))
        // Users
        .route("/api/v1/me", get(users::me))
        .route("/api/v1/me/public-profile", put(users::set_public_profile))
        .route("/api/v1/api-keys", post(users::create_api_key).get(users::list_api_keys))
        .route("/api/v1/api-keys/{id}", delete(users::delete_api_key))
        // Watcher
        .route("/api/v1/watch", post(watcher::watch).delete(watcher::unwatch))
        .route("/api/v1/watched", get(watcher::list))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let public = Router::new()
        .route("/health", get(health::health))
        .route("/api/v1/status", get(health::status))
        .route("/api/v1/card.svg", get(card::card_svg))
        .route("/api/v1/card/{username}", get(card::public_card_svg))
        .route(
            "/api/v1/register",
            post(register::register)
                .layer(middleware::from_fn(rate_limit::register_rate_limit)),
        );

    Router::new()
        .merge(authed)
        .merge(public)
        .fallback(web::static_handler)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
