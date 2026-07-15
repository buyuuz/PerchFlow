use crate::web::WebContext;
use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
    routing::get,
    Json, Router,
};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;

pub fn routes() -> Router<WebContext> {
    Router::new()
        .route("/processes/:name/logs", get(recent_logs))
        .route("/logs/stream/:name", get(stream_logs))
}

#[derive(Deserialize)]
struct LogQuery { lines: Option<usize> }

async fn recent_logs(
    State(ctx): State<WebContext>,
    Path(name): Path<String>,
    Query(q): Query<LogQuery>,
) -> Json<serde_json::Value> {
    let n = q.lines.unwrap_or(200);
    let rt = ctx.state.procs.read().await.get(&name).cloned();
    let lines = match rt {
        Some(rt) => rt.read().await.logs.recent(n),
        None => vec![],
    };
    Json(json!({ "name": name, "lines": lines }))
}

async fn stream_logs(
    State(ctx): State<WebContext>,
    Path(name): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let rt = ctx.state.procs.read().await.get(&name).cloned();
            let lines = match rt {
                Some(rt) => rt.read().await.logs.recent(50),
                None => vec![],
            };
            let payload = serde_json::json!({ "lines": lines }).to_string();
            yield Ok(Event::default().data(payload));
        }
    };
    Sse::new(stream)
}
