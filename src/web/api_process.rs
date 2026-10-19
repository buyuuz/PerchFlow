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
        .route("/processes/validate-cwd", post(validate_cwd))
        .route("/processes/:name", get(detail).put(update).delete(remove))
        .route("/processes/:name/alias", axum::routing::put(update_alias))
        .route("/processes/:name/cwd", axum::routing::put(update_cwd))
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
    if let Some(ref cwd) = def.cwd {
        if let Err(code) = verify_working_directory(cwd) {
            return Json(json!({ "ok": false, "error": code }));
        }
    }
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

async fn update(State(ctx): State<WebContext>, Path(name): Path<String>, Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    // Existing UI does not expose env/restart. Preserve them when omitted
    // instead of silently resetting user-maintained TOML settings.
    let has_env = payload.get("env").is_some();
    let has_restart = payload.get("restart").is_some();
    let Ok(mut def) = serde_json::from_value::<ProcessDef>(payload) else {
        return Json(json!({ "ok": false, "error": "invalid process fields" }));
    };
    // Legacy relative cwd values remain valid if unchanged. New/modified cwd
    // values must be absolute and must refer to an existing host directory.
    let cwd_unchanged = {
        let cfg = ctx.state.config.read().await;
        cfg.process.iter().any(|p| p.name == name && p.cwd == def.cwd)
    };
    if !cwd_unchanged {
        if let Some(ref cwd) = def.cwd {
            if let Err(code) = verify_working_directory(cwd) {
                return Json(json!({ "ok": false, "error": code }));
            }
        }
    }
    // Path name is authoritative; prevents runtime-map desync on rename.
    def.name = name.clone();
    let was_managed = ctx.supervisor.is_managed(&name).await;
    {
        let mut cfg = ctx.state.config.write().await;
        match cfg.process.iter_mut().find(|p| p.name == name) {
            Some(p) => {
                if !has_env { def.env = p.env.clone(); }
                if !has_restart { def.restart = p.restart.clone(); }
                *p = def;
            },
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

// The browser cannot inspect the filesystem of the PerchFlow host.
// Validate against the actual host here, behind the existing API token guard.
#[derive(Deserialize)]
struct WorkingDirectoryRequest {
    cwd: Option<String>,
}

fn normalize_cwd(cwd: Option<String>) -> Option<String> {
    cwd.map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn verify_working_directory(value: &str) -> std::result::Result<(), &'static str> {
    use std::path::Path;
    if value.chars().count() > 1024 {
        return Err("cwd_too_long");
    }
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err("cwd_not_absolute");
    }
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err("cwd_not_directory"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err("cwd_not_found"),
        Err(_) => Err("cwd_inaccessible"),
    }
}

async fn validate_cwd(Json(body): Json<WorkingDirectoryRequest>) -> Json<serde_json::Value> {
    match normalize_cwd(body.cwd) {
        Some(value) => match verify_working_directory(&value) {
            Ok(()) => Json(json!({ "ok": true })),
            Err(code) => Json(json!({ "ok": false, "error": code })),
        },
        None => Json(json!({ "ok": true })),
    }
}

// A cwd-only edit must preserve env, restart policy and all other TOML fields.
// An already running managed process is restarted so its new cwd takes effect.
async fn update_cwd(
    State(ctx): State<WebContext>,
    Path(name): Path<String>,
    Json(body): Json<WorkingDirectoryRequest>,
) -> Json<serde_json::Value> {
    let cwd = normalize_cwd(body.cwd);
    if let Some(ref value) = cwd {
        if let Err(code) = verify_working_directory(value) {
            return Json(json!({ "ok": false, "error": code }));
        }
    }
    let was_managed = ctx.supervisor.is_managed(&name).await;
    let changed = {
        let mut cfg = ctx.state.config.write().await;
        match cfg.process.iter_mut().find(|p| p.name == name) {
            Some(p) => {
                let changed = p.cwd != cwd;
                p.cwd = cwd;
                changed
            }
            None => return Json(json!({ "ok": false, "error": "not found" })),
        }
    };
    if !changed {
        return Json(json!({ "ok": true }));
    }
    if let Err(error) = persist(&ctx).await {
        return op_result(Err(error));
    }
    if was_managed {
        return op_result(ctx.supervisor.restart(&name).await);
    }
    Json(json!({ "ok": true }))
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
