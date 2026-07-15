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
}

async fn build_view(ctx: &WebContext, name: &str) -> Option<TaskView> {
    let cfg = ctx.state.config.read().await;
    let t = cfg.task.iter().find(|t| t.name == name)?;
    let rt = ctx.state.tasks.read().await.get(name).cloned();
    let (last_run, last_exit_code) = match rt {
        Some(r) => { let g = r.read().await; (g.last_run.map(|d| d.to_rfc3339()), g.last_exit_code) }
        None => (None, None),
    };
    Some(TaskView {
        name: t.name.clone(), schedule: t.schedule.clone(), cmd: t.cmd.clone(),
        args: t.args.clone(), cwd: t.cwd.clone(),
        enabled: t.enabled, last_run, last_exit_code,
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
            crate::core::state::TaskRuntime { last_run: None, last_exit_code: None },
        )),
    );
    op_result(persist(&ctx).await)
}

async fn update(State(ctx): State<WebContext>, Path(name): Path<String>, Json(def): Json<TaskDef>) -> Json<serde_json::Value> {
    if cron::Schedule::from_str(&def.schedule).is_err() {
        return Json(json!({ "ok": false, "error": "bad cron" }));
    }
    {
        let mut cfg = ctx.state.config.write().await;
        match cfg.task.iter_mut().find(|t| t.name == name) {
            Some(t) => *t = def,
            None => return Json(json!({ "ok": false, "error": "not found" })),
        }
    }
    op_result(persist(&ctx).await)
}

async fn remove(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    { ctx.state.config.write().await.task.retain(|t| t.name != name); }
    ctx.state.tasks.write().await.remove(&name);
    op_result(persist(&ctx).await)
}

async fn run_now(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    op_result(ctx.scheduler.run_once(&name).await)
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
