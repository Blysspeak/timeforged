use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

use timeforged_core::models::User;

use crate::app::AppState;
use crate::service::user_service;
use crate::storage::sqlite;

#[derive(Clone)]
pub struct AuthUser(pub User);

pub async fn auth_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let user = match api_key {
        Some(key) => user_service::authenticate(&state.db, &key)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?,
        None => {
            // Auto-auth for localhost (dashboard) — use first user
            if addr.ip().is_loopback() {
                sqlite::get_first_user(&state.db)
                    .await
                    .map_err(|_| StatusCode::UNAUTHORIZED)?
                    .ok_or(StatusCode::UNAUTHORIZED)?
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    };

    req.extensions_mut().insert(AuthUser(user));
    Ok(next.run(req).await)
}
