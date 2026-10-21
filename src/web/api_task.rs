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
        .route("/tasks/:name/stop", post(stop_task))
        .route("/tasks/:name/history", get(history))
        .route("/tasks/:name/log/:run_id", get(log))
        .route("/tasks/:name/files/:run_id", get(files))
}

#[derive(Serialize)]
pub struct TaskView {
    pub name: String,
    pub project: Option<String>,
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
    pub pid: Option<u32>,
    pub cpu_percent: f32,
    pub mem_bytes: u64,
    pub uptime_secs: u64,
}

async fn build_view(ctx: &WebContext, name: &str) -> Option<TaskView> {
    let cfg = ctx.state.config.read().await;
    let t = cfg.task.iter().find(|t| t.name == name)?;
    let rt = ctx.state.tasks.read().await.get(name).cloned();
    let (last_run, last_exit_code, status, started_at, finished_at, run_id, pid, cpu_percent, mem_bytes, uptime_secs) = match rt {
        Some(r) => { let g = r.read().await; (g.last_run.map(|d| d.to_rfc3339()), g.last_exit_code, g.status.clone(), g.started_at.map(|d| d.to_rfc3339()), g.finished_at.map(|d| d.to_rfc3339()), g.run_id.clone(), g.pid, g.cpu_percent, g.mem_bytes,
            if g.status == "running" { g.started_at.map(|d| (chrono::Utc::now() - d).num_seconds().max(0) as u64).unwrap_or(0) } else { 0 }) }
        None => (None, None, "idle".into(), None, None, None, None, 0.0, 0, 0),
    };
    Some(TaskView {
        name: t.name.clone(),
        project: cfg.project.iter().find(|p| p.tasks.contains(&t.name)).map(|p| p.name.clone()),
        schedule: t.schedule.clone(), cmd: t.cmd.clone(),
        args: t.args.clone(), cwd: t.cwd.clone(),
        enabled: t.enabled, last_run, last_exit_code, status, started_at, finished_at, run_id, timeout_secs: t.timeout_secs,
        pid, cpu_percent, mem_bytes, uptime_secs,
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

/// Persist task configuration *before* exposing its changes in memory.
/// A disk write failure must not change the active scheduler or task registry.
fn save_task_config(
    current: &mut crate::config::model::Config,
    next: crate::config::model::Config,
    ctx: &WebContext,
) -> crate::error::Result<()> {
    writer::save(&ctx.state.config_path, &next)?;
    *current = next;
    Ok(())
}

async fn create(State(ctx): State<WebContext>, Json(def): Json<TaskDef>) -> Json<serde_json::Value> {
    if cron::Schedule::from_str(&def.schedule).is_err() {
        return Json(json!({"ok":false,"error":"bad cron"}));
    }
    let mut cfg = ctx.state.config.write().await;
    if cfg.task.iter().any(|t| t.name == def.name) {
        return Json(json!({"ok":false,"error":"name exists"}));
    }
    let mut next = cfg.clone();
    next.task.push(def.clone());
    let result = save_task_config(&mut cfg, next, &ctx);
    drop(cfg);
    if result.is_err() { return op_result(result); }
    ctx.state.tasks.write().await.insert(
        def.name.clone(),
        std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::core::state::TaskRuntime {
                last_run:None, last_exit_code:None, status:"idle".into(),
                started_at:None, finished_at:None, run_id:None,
                pid:None, cpu_percent:0.0, mem_bytes:0,
            }
        )),
    );
    ctx.scheduler.reload().await;
    Json(json!({"ok":true}))
}

async fn update(
    State(ctx): State<WebContext>, Path(name): Path<String>, Json(def): Json<TaskDef>,
) -> Json<serde_json::Value> {
    if cron::Schedule::from_str(&def.schedule).is_err() {
        return Json(json!({"ok":false,"error":"bad cron"}));
    }
    if ctx.scheduler.is_running(&name).await {
        return Json(json!({"ok":false,"error":"task is running; stop it before editing"}));
    }
    let new_name = def.name.clone();
    let mut cfg = ctx.state.config.write().await;
    if cfg.project.iter().any(|p| p.tasks.contains(&name)) && name != new_name {
        return Json(json!({"ok":false,"error":"task belongs to a project; unlink it before renaming"}));
    }
    if new_name != name && cfg.task.iter().any(|t| t.name == new_name) {
        return Json(json!({"ok":false,"error":"name exists"}));
    }
    let mut next = cfg.clone();
    let Some(task) = next.task.iter_mut().find(|t| t.name == name) else {
        return Json(json!({"ok":false,"error":"not found"}));
    };
    *task = def;
    let result = save_task_config(&mut cfg, next, &ctx);
    drop(cfg);
    if result.is_err() { return op_result(result); }
    if new_name != name {
        let mut map = ctx.state.tasks.write().await;
        if let Some(rt) = map.remove(&name) {
            map.insert(new_name, rt);
        }
    }
    ctx.scheduler.reload().await;
    Json(json!({"ok":true}))
}

async fn remove(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    if ctx.scheduler.is_running(&name).await {
        return Json(json!({"ok":false,"error":"task is running; stop it before deleting"}));
    }
    let mut cfg = ctx.state.config.write().await;
    if cfg.project.iter().any(|p| p.tasks.contains(&name)) {
        return Json(json!({"ok":false,"error":"task belongs to a project; unlink it before deleting"}));
    }
    if cfg.task.iter().all(|t| t.name != name) {
        return Json(json!({"ok":false,"error":"not found"}));
    }
    let mut next = cfg.clone();
    next.task.retain(|t| t.name != name);
    let result = save_task_config(&mut cfg, next, &ctx);
    drop(cfg);
    if result.is_err() { return op_result(result); }
    ctx.state.tasks.write().await.remove(&name);
    ctx.scheduler.reload().await;
    Json(json!({"ok":true}))
}

async fn run_now(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    match ctx.scheduler.start_now(&name).await {
        Ok(run_id) => Json(json!({"ok":true,"run_id":run_id})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}

async fn stop_task(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    match ctx.scheduler.cancel_task(&name).await {
        Ok(run_id) => Json(json!({"ok":true,"run_id":run_id,"requested":true})),
        Err(e) => match build_view(&ctx, &name).await {
            Some(view) if view.status != "running" => Json(json!({
                "ok":true,"already_finished":true,"status":view.status,"run_id":view.run_id
            })),
            Some(_) => Json(json!({"ok":false,"error":e.to_string()})),
            None => Json(json!({"ok":false,"error":"task not found"})),
        }
    }
}

async fn history(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    if ctx.state.config.read().await.task.iter().all(|t| t.name != name) {
        return Json(json!({"ok":false,"error":"not found"}));
    }
    let current_run_id = match ctx.state.tasks.read().await.get(&name).cloned() {
        Some(task) => {
            let rt = task.read().await;
            if rt.status == "running" { rt.run_id.clone() } else { None }
        }
        None => None,
    };
    let base = ctx.state.config_path.parent().unwrap_or(std::path::Path::new(".")).join("task-logs").join("history.jsonl");
    use std::io::BufRead;
    let mut entries: std::collections::VecDeque<serde_json::Value> = std::collections::VecDeque::new();
    if let Ok(file) = std::fs::File::open(base) {
        for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
            if let Ok(record) = serde_json::from_str::<serde_json::Value>(&line) {
                if record.get("task").and_then(|t| t.as_str()) == Some(name.as_str()) {
                    entries.push_back(record);
                    if entries.len() > 300 { entries.pop_front(); }
                }
            }
        }
    }
    let mut ids = std::collections::HashSet::new();
    let mut runs: Vec<_> = entries.into_iter().rev()
        .filter(|r| r.get("run_id").and_then(|v| v.as_str()).is_some_and(|id| ids.insert(id.to_owned())))
        .take(100).map(|mut r| {
            if r.get("status").and_then(|v| v.as_str()) == Some("running")
                && r.get("run_id").and_then(|v| v.as_str()) != current_run_id.as_deref() {
                r["status"] = serde_json::Value::String("interrupted".into());
            }
            r
        }).collect();
    // Body retention is intentionally separate from the persistent history.
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let log_dir = ctx.state.config_path.parent().unwrap_or(std::path::Path::new(".")).join("task-logs");
    for entry in &mut runs {
        let available = entry.get("run_id").and_then(|v|v.as_str())
            .filter(|id| id.chars().all(|c|c.is_ascii_alphanumeric() || c == '-'))
            .is_some_and(|id| log_dir.join(format!("{safe}-{id}.log")).is_file());
        entry["log_available"] = json!(available);
    }
    Json(json!({"ok":true,"runs":runs}))
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
    use std::io::{Read, Seek, SeekFrom};
    match std::fs::File::open(path) {
        Ok(mut file) => {
            let length = file.metadata().map(|m| m.len()).unwrap_or(0);
            let start = length.saturating_sub(131_072);
            if file.seek(SeekFrom::Start(start)).is_err() {
                return Json(json!({"ok":false,"error":"log seek failed"}));
            }
            let mut buffer = Vec::new();
            match file.take(131_072).read_to_end(&mut buffer) {
                Ok(_) => Json(json!({"ok":true,"content":String::from_utf8_lossy(&buffer),"truncated":start>0})),
                Err(_) => Json(json!({"ok":false,"error":"log read failed"})),
            }
        }
        Err(_) => Json(json!({"ok":false,"error":"log not found"})),
    }
}

fn op_result(r: crate::error::Result<()>) -> Json<serde_json::Value> {
    match r {
        Ok(_) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}


/// Per-run file change metadata; protected by the same bearer auth as task logs.
/// No file body, account password or API key is ever returned by this route.
async fn files(
    State(ctx): State<WebContext>,
    Path((name, run_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if ctx.state.config.read().await.task.iter().all(|t| t.name != name) {
        return Json(json!({"ok":false,"error":"task not found"}));
    }
    let base = ctx.state.config_path.parent().unwrap_or(std::path::Path::new(".")).join("task-logs");
    match crate::core::task_files::read(&base, &name, &run_id) {
        Ok(record) => Json(json!({"ok":true,"record":record})),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound =>
            Json(json!({"ok":false,"error":"file metadata not recorded for this run"})),
        Err(_) => Json(json!({"ok":false,"error":"file metadata unavailable"})),
    }
}
