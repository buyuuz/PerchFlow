use crate::core::state::ProcStatus;
use crate::web::WebContext;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;

pub fn routes() -> Router<WebContext> {
    Router::new().route("/processes/:name/logs", get(recent_logs))
}

/// `/logs/stream/:name` cannot use the header-auth middleware because
/// browser `EventSource` cannot send an `Authorization` header. It is
/// mounted separately and validates `?token=` itself.
pub fn stream_routes() -> Router<WebContext> {
    Router::new().route("/logs/stream/:name", get(stream_logs))
}

#[derive(Deserialize)]
struct LogQuery { lines: Option<usize> }

#[derive(Deserialize)]
struct StreamQuery { token: String }

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
    Query(q): Query<StreamQuery>,
) -> Response {
    let expected = ctx.state.config.read().await.server.token.clone();
    if q.token != expected {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    sse_stream(ctx, name).into_response()
}

/// SSE 流:进程 Running/Restarting 时每秒推送 recent(50) 增量;一旦进入
/// Stopped/Crashed 终态,推送一次 `{"ended":true,"status":"..."}`
/// 后正常结束流,避免进程停止后日志区还在反复刷旧 ring buffer 内容。
fn sse_stream(ctx: WebContext, name: String) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let rt = ctx.state.procs.read().await.get(&name).cloned();
            let (lines, status) = match rt {
                Some(rt) => {
                    let g = rt.read().await;
                    (g.logs.recent(50), g.status)
                }
                None => (vec![], ProcStatus::Stopped),
            };
            match status {
                ProcStatus::Running | ProcStatus::Restarting => {
                    let payload = json!({ "lines": lines }).to_string();
                    yield Ok(Event::default().data(payload));
                }
                terminal => {
                    // 终态:推一次 ended 告知前端关流,然后结束本 SSE 流。
                    let payload = json!({ "ended": true, "status": status_label(terminal) }).to_string();
                    yield Ok(Event::default().data(payload));
                    break;
                }
            }
        }
    };
    Sse::new(stream)
}

fn status_label(s: ProcStatus) -> &'static str {
    match s {
        ProcStatus::Running => "running",
        ProcStatus::Restarting => "restarting",
        ProcStatus::Stopped => "stopped",
        ProcStatus::Crashed => "crashed",
    }
}
