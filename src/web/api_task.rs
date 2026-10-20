use crate::config::{model::TaskDef, writer};
use crate::web::WebContext;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::str::FromStr;

pub fn routes() -> Router<WebContext> {
    Router::new()
        .route("/tasks", get(list).post(create))
        .route("/tasks/:name", get(detail).put(update).delete(remove))
        .route("/tasks/:name/run", post(run_now))
        .route("/tasks/:name/history", get(history))
        .route("/tasks/:name/log/:run_id", get(log))
}

#[derive(Serialize)]
pub struct TaskView {
    pub name: String,
    pub schedule: String,
    pub cmd: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub last_exit_code: Option<i32>,
    pub status: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub run_id: Option<String>,
    pub timeout_secs: u64,
}

async fn build_view(ctx: &WebContext, name: &str) -> Option<TaskView> {
    let cfg = ctx.state.config.read().await;
    let t = cfg.task.iter().find(|t| t.name == name)?;
    let rt = ctx.state.tasks.read().await.get(name).cloned();
    let (last_run, last_exit_code, status, started_at, finished_at, run_id) = match rt {
        Some(r) => { let g = r.read().await; (g.last_run.map(|d| d.to_rfc3339()), g.last_exit_code, g.status.clone(), g.started_at.map(|d| d.to_rfc3339()), g.finished_at.map(|d| d.to_rfc3339()), g.run_id.clone()) }
        None => (None, None, "idle".into(), None, None, None),
    };
    Some(TaskView {
        name: t.name.clone(), schedule: t.schedule.clone(), cmd: t.cmd.clone(),
        args: t.args.clone(), cwd: t.cwd.clone(),
        enabled: t.enabled, last_run, last_exit_code, status, started_at, finished_at, run_id, timeout_secs: t.timeout_secs,
    })
}

async fn list(State(ctx): State<WebContext>) -> Json<serde_json::Value> {
    let names: Vec<String> = ctx.state.config.read().await.task.iter().map(|t| t.name.clone()).collect();
    let mut views = Vec::new();
    for n in names { if let Some(v) = build_view(&ctx, &n).await { views.push(v); } }
    Json(json!({ "tasks": views }))
}

async fn detail(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    match build_view(&ctx, &name).await {
        Some(v) => Json(json!(v)),
        None => Json(json!({ "ok": false, "error": "not found" })),
    }
}

async fn create(State(ctx): State<WebContext>, Json(def): Json<TaskDef>) -> Json<serde_json::Value> {
    if cron::Schedule::from_str(&def.schedule).is_err() {
        return Json(json!({ "ok": false, "error": "bad cron" }));
    }
    {
        let mut cfg = ctx.state.config.write().await;
        if cfg.task.iter().any(|t| t.name == def.name) {
            return Json(json!({ "ok": false, "error": "name exists" }));
        }
        cfg.task.push(def.clone());
    }
    ctx.state.tasks.write().await.insert(
        def.name.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::core::state::TaskRuntime { last_run: None, last_exit_code: None, status: "idle".into(), started_at: None, finished_at: None, run_id: None },
        )),
    );
    let r = persist(&ctx).await;
    ctx.scheduler.reload().await;
    op_result(r)
}

async fn update(State(ctx): State<WebContext>, Path(name): Path<String>, Json(def): Json<TaskDef>) -> Json<serde_json::Value> {
    if cron::Schedule::from_str(&def.schedule).is_err() {
        return Json(json!({ "ok": false, "error": "bad cron" }));
    }
    let old_name = name;
    let new_name = def.name.clone();
    {
        let mut cfg = ctx.state.config.write().await;
        if new_name != old_name && cfg.task.iter().any(|t| t.name == new_name) {
            return Json(json!({ "ok": false, "error": "name exists" }));
        }
        match cfg.task.iter_mut().find(|t| t.name == old_name) {
            Some(t) => *t = def,
            None => return Json(json!({ "ok": false, "error": "not found" })),
        }
    }
    // 改名时迁移运行期状态到新 key,否则 run_once 按新名字找不到
    // TaskRuntime,last_run/last_exit_code 会静默丢失。
    if new_name != old_name {
        let mut map = ctx.state.tasks.write().await;
        if let Some(rt) = map.remove(&old_name) {
            map.insert(new_name.clone(), rt);
        }
    }
    let r = persist(&ctx).await;
    ctx.scheduler.reload().await;
    op_result(r)
}

async fn remove(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    { ctx.state.config.write().await.task.retain(|t| t.name != name); }
    ctx.state.tasks.write().await.remove(&name);
    let r = persist(&ctx).await;
    ctx.scheduler.reload().await;
    op_result(r)
}

async fn run_now(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    let task_name = name.clone();
    tracing::info!("task '{}' triggered (manual)", task_name);
    let r = ctx.scheduler.run_once(&name).await;
    match &r {
        Ok(()) => tracing::info!("task '{}' completed (manual)", task_name),
        Err(e) => tracing::warn!("task '{}' failed (manual): {e}", task_name),
    }
    op_result(r)
}

async fn history(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    if ctx.state.config.read().await.task.iter().all(|t| t.name != name) {
        return Json(json!({"ok":false,"error":"not found"}));
    }
    let base = ctx.state.config_path.parent().unwrap_or(std::path::Path::new(".")).join("task-logs").join("history.jsonl");
    let entries: Vec<serde_json::Value> = std::fs::read_to_string(base).unwrap_or_default().lines()
        .filter_map(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .filter(|r| r.get("task").and_then(|t| t.as_str()) == Some(name.as_str()))
        .collect();
    Json(json!({"ok":true,"runs":entries.into_iter().rev().take(100).collect::<Vec<_>>()}))
}

async fn log(State(ctx): State<WebContext>, Path((name, run_id)): Path<(String, String)>) -> Json<serde_json::Value> {
    if ctx.state.config.read().await.task.iter().all(|t| t.name != name) {
        return Json(json!({"ok":false,"error":"not found"}));
    }
    if !run_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Json(json!({"ok":false,"error":"invalid run id"}));
    }
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let path = ctx.state.config_path.parent().unwrap_or(std::path::Path::new(".")).join("task-logs").join(format!("{safe}-{run_id}.log"));
    match std::fs::read_to_string(path) {
        Ok(content) => Json(json!({"ok":true,"content":content})),
        Err(_) => Json(json!({"ok":false,"error":"log not found"})),
    }
}

async fn persist(ctx: &WebContext) -> crate::error::Result<()> {
    let cfg = ctx.state.config.read().await;
    writer::save(&ctx.state.config_path, &cfg)
}

fn op_result(r: crate::error::Result<()>) -> Json<serde_json::Value> {
    match r {
        Ok(_) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}
