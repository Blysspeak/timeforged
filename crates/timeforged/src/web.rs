use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

use crate::app::AppState;

#[derive(Embed)]
#[folder = "web/dist"]
struct WebAssets;

pub async fn static_handler(uri: Uri, State(state): State<AppState>) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the exact file
    if let Some(file) = WebAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .body(Body::from(file.data.to_vec()))
            .unwrap()
    } else {
        // SPA fallback: serve index.html with injected API key
        index_html_with_key(&state).await.into_response()
    }
}

async fn index_html_with_key(state: &AppState) -> impl IntoResponse {
    match WebAssets::get("index.html") {
        Some(file) => {
            let mut html = String::from_utf8_lossy(&file.data).to_string();
            // Inject API key from cli.toml so dashboard auto-authenticates
            let cli = timeforged_core::config::CliConfig::load();
            if let Some(key) = cli.api_key {
                let script = format!(
                    r#"<script>if(!localStorage.getItem('tf_api_key'))localStorage.setItem('tf_api_key','{key}');</script>"#
                );
                html = html.replace("</head>", &format!("{script}</head>"));
            }
            Html(html).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Dashboard not built").into_response(),
    }
}
