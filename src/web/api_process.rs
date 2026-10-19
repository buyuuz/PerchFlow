use crate::config::{model::ProcessDef, writer};
use crate::core::state::{ProcRuntime, ProcStatus, SharedState};
use crate::web::WebContext;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub fn routes() -> Router<WebContext> {
    Router::new()
        .route("/processes", get(list).post(create))
        .route("/processes/:name", get(detail).put(update).delete(remove))
        .route("/processes/:name/alias", axum::routing::put(update_alias))
        .route("/processes/:name/start", post(start))
        .route("/processes/:name/stop", post(stop))
        .route("/processes/:name/restart", post(restart))
}

#[derive(Serialize)]
pub struct ProcView {
    pub alias_zh: Option<String>,
    pub name: String,
    pub status: ProcStatus,
    pub pid: Option<u32>,
    pub uptime_secs: u64,
    pub restart_count: u32,
    pub cpu_percent: f32,
    pub mem_bytes: u64,
    pub cmd: String,      // 展示用:cmd 与 args 拼接后的完整命令行
    pub cmd_raw: String,  // 编辑用:未拼接 args 的原始 cmd
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub autostart: bool,
}

async fn build_view(state: &SharedState, name: &str) -> Option<ProcView> {
    let cfg = state.config.read().await;
    let def = cfg.process.iter().find(|p| p.name == name)?;
    let cmd = if def.args.is_empty() { def.cmd.clone() }
        else { format!("{} {}", def.cmd, def.args.join(" ")) };
    let alias_zh = def.alias_zh.clone();
    let cmd_raw = def.cmd.clone();
    let args = def.args.clone();
    let cwd = def.cwd.clone();
    let autostart = def.autostart;
    drop(cfg);
    let rt_arc = state.procs.read().await.get(name).cloned()?;
    let rt = rt_arc.read().await;
    let uptime = rt.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
    Some(ProcView {
        name: name.to_string(), alias_zh, status: rt.status, pid: rt.pid,
        uptime_secs: uptime, restart_count: rt.restart_count,
        cpu_percent: rt.cpu_percent, mem_bytes: rt.mem_bytes, cmd, cmd_raw,
        args, cwd, autostart,
    })
}

async fn list(State(ctx): State<WebContext>) -> Json<serde_json::Value> {
    let names: Vec<String> = ctx.state.config.read().await.process.iter().map(|p| p.name.clone()).collect();
    let mut views = Vec::new();
    for n in names {
        if let Some(v) = build_view(&ctx.state, &n).await { views.push(v); }
    }
    Json(json!({ "processes": views }))
}

async fn detail(State(ctx): State<WebContext>, Path(name): Path<String>) -> Result<Json<ProcView>, StatusCode> {
    build_view(&ctx.state, &name).await.map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn start(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    let r = ctx.supervisor.start(&name).await;
    op_result(r)
}
async fn stop(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    op_result(ctx.supervisor.stop(&name).await)
}
async fn restart(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    op_result(ctx.supervisor.restart(&name).await)
}

fn op_result(r: crate::error::Result<()>) -> Json<serde_json::Value> {
    match r {
        Ok(_) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn persist(ctx: &WebContext) -> crate::error::Result<()> {
    let cfg = ctx.state.config.read().await;
    writer::save(&ctx.state.config_path, &cfg)
}

async fn create(State(ctx): State<WebContext>, Json(def): Json<ProcessDef>) -> Json<serde_json::Value> {
    {
        let mut cfg = ctx.state.config.write().await;
        if cfg.process.iter().any(|p| p.name == def.name) {
            return Json(json!({ "ok": false, "error": "name exists" }));
        }
        cfg.process.push(def.clone());
    }
    ctx.state.procs.write().await.insert(
        def.name.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(ProcRuntime::new())),
    );
    let r = persist(&ctx).await;
    if def.autostart {
        if let Err(e) = ctx.supervisor.start(&def.name).await {
            return Json(json!({ "ok": false, "error": e.to_string() }));
        }
    }
    op_result(r)
}

async fn update(State(ctx): State<WebContext>, Path(name): Path<String>, Json(mut def): Json<ProcessDef>) -> Json<serde_json::Value> {
    // Path name is authoritative; prevents runtime-map desync on rename.
    def.name = name.clone();
    let was_managed = ctx.supervisor.is_managed(&name).await;
    {
        let mut cfg = ctx.state.config.write().await;
        match cfg.process.iter_mut().find(|p| p.name == name) {
            Some(p) => *p = def,
            None => return Json(json!({ "ok": false, "error": "not found" })),
        }
    }
    let r = persist(&ctx).await;
    if was_managed {
        if let Err(e) = ctx.supervisor.restart(&name).await {
            return Json(json!({ "ok": false, "error": e.to_string() }));
        }
    }
    op_result(r)
}

// Display-only Chinese alias. Update without touching the supervisor or a running process.
#[derive(Deserialize)]
struct AliasUpdate {
    alias_zh: Option<String>,
}

async fn update_alias(
    State(ctx): State<WebContext>,
    Path(name): Path<String>,
    Json(body): Json<AliasUpdate>,
) -> Json<serde_json::Value> {
    let alias = body.alias_zh.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if alias.as_ref().is_some_and(|s| s.chars().count() > 80) {
        return Json(json!({ "ok": false, "error": "alias too long (max 80 characters)" }));
    }
    {
        let mut cfg = ctx.state.config.write().await;
        match cfg.process.iter_mut().find(|p| p.name == name) {
            Some(p) => p.alias_zh = alias,
            None => return Json(json!({ "ok": false, "error": "not found" })),
        }
    }
    op_result(persist(&ctx).await)
}

async fn remove(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    let _ = ctx.supervisor.stop(&name).await;
    {
        let mut cfg = ctx.state.config.write().await;
        cfg.process.retain(|p| p.name != name);
    }
    ctx.state.procs.write().await.remove(&name);
    op_result(persist(&ctx).await)
}
