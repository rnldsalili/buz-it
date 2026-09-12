use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tauri::{AssetResolver, Runtime};

/// Serve only exact embedded files; Tauri's resolver otherwise falls back to HTML.
pub fn router<R: Runtime>(resolver: AssetResolver<R>) -> Router {
    let paths: HashSet<String> = resolver.iter().map(|(path, _)| path.to_string()).collect();
    let paths = Arc::new(paths);
    let resolver = Arc::new(resolver);
    Router::new().fallback_service(get(move |uri: Uri| {
        let resolver = resolver.clone();
        let paths = paths.clone();
        async move {
            let path = match uri.path() {
                "/" => "/player.html",
                "/board" => "/board.html",
                "/host" | "/host.html" => return StatusCode::NOT_FOUND.into_response(),
                path => path,
            };
            if !paths.contains(path) {
                return StatusCode::NOT_FOUND.into_response();
            }
            match resolver.get(path.to_owned()) {
                Some(asset) => asset_response(asset),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }))
}

fn asset_response(asset: tauri::Asset) -> Response {
    ([(header::CONTENT_TYPE, asset.mime_type)], asset.bytes).into_response()
}
