use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use timeforged_core::models::User;

use crate::app::AppState;
use crate::service::user_service;

#[derive(Clone)]
pub struct AuthUser(pub User);

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = req
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = user_service::authenticate(&state.db, &api_key)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(AuthUser(user));
    Ok(next.run(req).await)
}
