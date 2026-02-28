use std::path::PathBuf;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use timeforged_core::api::{
    ErrorResponse, UnwatchRequest, WatchActionResponse, WatchRequest, WatchedDirResponse,
    WatchedResponse,
};
use timeforged_core::config::WatchedRegistry;
use timeforged_core::error::AppError;

use crate::app::AppState;
use crate::watcher::WatcherCommand;

pub async fn watch(
    State(state): State<AppState>,
    Json(req): Json<WatchRequest>,
) -> impl IntoResponse {
    match do_watch(state, req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn unwatch(
    State(state): State<AppState>,
    Json(req): Json<UnwatchRequest>,
) -> impl IntoResponse {
    match do_unwatch(state, req).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => error_response(e),
    }
}

pub async fn list(State(_state): State<AppState>) -> impl IntoResponse {
    let registry = WatchedRegistry::load();
    let dirs = registry
        .list()
        .iter()
        .map(|d| WatchedDirResponse {
            path: d.path.clone(),
            added_at: d.added_at,
        })
        .collect();

    (StatusCode::OK, Json(WatchedResponse { dirs })).into_response()
}

async fn do_watch(state: AppState, req: WatchRequest) -> Result<WatchActionResponse, AppError> {
    let path = PathBuf::from(&req.path);
    let canonical = path
        .canonicalize()
        .map_err(|e| AppError::BadRequest(format!("invalid path: {e}")))?;

    if !canonical.is_dir() {
        return Err(AppError::BadRequest("path is not a directory".into()));
    }

    let path_str = canonical.to_string_lossy().to_string();

    let mut registry = WatchedRegistry::load();
    let added = registry.add(path_str.clone());

    if added {
        registry
            .save()
            .map_err(|e| AppError::Internal(format!("failed to save registry: {e}")))?;

        let _ = state
            .watcher_tx
            .send(WatcherCommand::Watch(canonical))
            .await;

        Ok(WatchActionResponse {
            message: format!("now watching {path_str}"),
        })
    } else {
        Ok(WatchActionResponse {
            message: format!("already watching {path_str}"),
        })
    }
}

async fn do_unwatch(
    state: AppState,
    req: UnwatchRequest,
) -> Result<WatchActionResponse, AppError> {
    let path = PathBuf::from(&req.path);
    let path_str = path
        .canonicalize()
        .unwrap_or_else(|_| path.clone())
        .to_string_lossy()
        .to_string();

    let mut registry = WatchedRegistry::load();
    let removed = registry.remove(&path_str);

    if removed {
        registry
            .save()
            .map_err(|e| AppError::Internal(format!("failed to save registry: {e}")))?;

        let _ = state
            .watcher_tx
            .send(WatcherCommand::Unwatch(PathBuf::from(&path_str)))
            .await;

        Ok(WatchActionResponse {
            message: format!("stopped watching {path_str}"),
        })
    } else {
        Err(AppError::NotFound(format!(
            "{path_str} is not being watched"
        )))
    }
}

fn error_response(e: AppError) -> axum::response::Response {
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
