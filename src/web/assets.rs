use crate::web::WebContext;
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/"]
struct Assets;

pub fn routes() -> Router<WebContext> {
    Router::new()
        .route("/", get(|| async { page("login.html") }))
        .route("/processes", get(|| async { page("processes.html") }))
        .route("/process-detail", get(|| async { page("process-detail.html") }))
        .route("/tasks", get(|| async { page("tasks.html") }))
        .route("/assets/*path", get(asset))
}

fn page(name: &str) -> Response {
    match Assets::get(name) {
        Some(f) => {
            let html_str = String::from_utf8_lossy(&f.data);
            let replaced = html_str.replace("{{VERSION}}", env!("CARGO_PKG_VERSION"));
            Html(replaced).into_response()
        },
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn asset(Path(path): Path<String>) -> Response {
    let full = format!("assets/{path}");
    match Assets::get(&full) {
        Some(f) => {
            let mime = mime_guess::from_path(&full).first_or_octet_stream();
            // Font assets are immutable between UI edits; Material Symbols alone
            // is over 3 MiB, so avoid redownloading it on each navigation.
            let cache = if full.ends_with(".woff2") {
                "private, max-age=86400"
            } else {
                "no-cache"
            };
            (
                [
                    (header::CONTENT_TYPE, mime.as_ref()),
                    (header::CACHE_CONTROL, cache),
                ],
                f.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
