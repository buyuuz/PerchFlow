use crate::config::writer;
use crate::web::WebContext;
use axum::{extract::State, routing::{get, put}, Json, Router};
use serde::Deserialize;
use serde_json::json;

const VALID_LANGS: [&str; 5] = ["zh", "en", "es", "ja", "pt"];

/// Authenticated: changing server config requires the token. Only PUT here;
/// GET lives in `public_routes()` so the two merge without a method clash.
pub fn routes() -> Router<WebContext> {
    Router::new().route("/lang", put(set_lang))
}

/// Unauthenticated: the login page (pre-token) needs to read the default
/// language before the user has a token. Mirrors api_logs::stream_routes().
pub fn public_routes() -> Router<WebContext> {
    Router::new().route("/lang", get(get_lang))
}

#[derive(Deserialize)]
struct LangBody { lang: String }

async fn get_lang(State(ctx): State<WebContext>) -> Json<serde_json::Value> {
    let lang = ctx.state.config.read().await.server.lang.clone();
    Json(json!({ "lang": lang }))
}

async fn set_lang(State(ctx): State<WebContext>, Json(body): Json<LangBody>) -> Json<serde_json::Value> {
    if !VALID_LANGS.contains(&body.lang.as_str()) {
        return Json(json!({ "ok": false, "error": "invalid lang" }));
    }
    { ctx.state.config.write().await.server.lang = body.lang; }
    let r = persist(&ctx).await;
    match r {
        Ok(_) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn persist(ctx: &WebContext) -> crate::error::Result<()> {
    let cfg = ctx.state.config.read().await;
    writer::save(&ctx.state.config_path, &cfg)
}
