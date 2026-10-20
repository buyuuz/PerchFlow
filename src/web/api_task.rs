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
        name: t.name.clone(),
        project: cfg.project.iter().find(|p| p.tasks.contains(&t.name)).map(|p| p.name.clone()),
        schedule: t.schedule.clone(), cmd: t.cmd.clone(),
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
    if ctx.scheduler.is_running(&old_name).await {
        return Json(json!({"ok":false,"error":"task is running; stop it before editing"}));
    }
    let new_name = def.name.clone();
    if new_name != old_name && ctx.state.config.read().await.project.iter().any(|p| p.tasks.contains(&old_name)) {
        return Json(json!({"ok":false,"error":"task belongs to a project; unlink it before renaming"}));
    }
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
    if ctx.scheduler.is_running(&name).await {
        return Json(json!({"ok":false,"error":"task is running; stop it before deleting"}));
    }
    if ctx.state.config.read().await.project.iter().any(|p| p.tasks.contains(&name)) {
        return Json(json!({"ok":false,"error":"task belongs to a project; unlink it before deleting"}));
    }
    { ctx.state.config.write().await.task.retain(|t| t.name != name); }
    ctx.state.tasks.write().await.remove(&name);
    let r = persist(&ctx).await;
    ctx.scheduler.reload().await;
    op_result(r)
}

async fn run_now(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    match ctx.scheduler.start_now(&name).await {
        Ok(run_id) => Json(json!({"ok":true,"run_id":run_id})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
    }
}

async fn stop_task(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<serde_json::Value> {
    match ctx.scheduler.cancel_task(&name).await {
        Ok(run_id) => Json(json!({"ok":true,"run_id":run_id})),
        Err(e) => Json(json!({"ok":false,"error":e.to_string()})),
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
    let runs: Vec<_> = entries.into_iter().rev()
        .filter(|r| r.get("run_id").and_then(|v| v.as_str()).is_some_and(|id| ids.insert(id.to_owned())))
        .take(100).map(|mut r| {
            if r.get("status").and_then(|v| v.as_str()) == Some("running")
                && r.get("run_id").and_then(|v| v.as_str()) != current_run_id.as_deref() {
                r["status"] = serde_json::Value::String("interrupted".into());
            }
            r
        }).collect();
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
