//! Project registry over the existing task scheduler. No file content access.
//! A project is metadata: an original working directory, existing task links,
//! and relative input/output file references. Deleting a project never deletes
//! task definitions, Python scripts, files or run logs.
use crate::config::{self, model::{ProjectDef, TaskDef}, writer};
use serde::Deserialize;
use std::str::FromStr;
use crate::web::WebContext;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use std::path::Path as FsPath;
use std::time::UNIX_EPOCH;

pub fn routes() -> Router<WebContext> {
    Router::new()
        .route("/projects", get(list).post(create))
        .route("/projects/:name", get(detail).put(update).delete(remove))
        .route("/projects/:name/python-task", axum::routing::post(add_python_task))
        .route("/projects/:name/tasks/:task/files", get(task_file_candidates))
        .route("/projects/:name/tasks/:task/open-file", axum::routing::post(open_task_file))
        .route("/projects/:name/preflight", get(preflight_status))
        .route("/projects/:name/tasks/:task/schedule", axum::routing::put(set_project_task_schedule))
}

async fn list(State(ctx): State<WebContext>) -> Json<Value> {
    let projects = ctx.state.config.read().await.project.clone();
    Json(json!({"projects":projects}))
}

async fn detail(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<Value> {
    let cfg = ctx.state.config.read().await;
    match cfg.project.iter().find(|p| p.name == name) {
        Some(project) => Json(json!({
            "ok":true,
            "project":project,
            "files": file_statuses(project),
        })),
        None => Json(json!({"ok":false,"error":"not found"})),
    }
}

// Metadata only; never open/read files (could contain credentials).
// Symlinks and paths outside the project root are never traversed.
fn file_statuses(project: &ProjectDef) -> Vec<Value> {
    let root = FsPath::new(&project.root);
    let canonical_root = root.canonicalize().ok();
    project.inputs.iter().map(|p| ("input", p))
        .chain(project.outputs.iter().map(|p| ("output", p)))
        .chain(project.output_dirs.iter().map(|p| ("output_dir", p)))
        .map(|(role, relative)| {
            let mut result = json!({
                "role":role,"path":relative,"exists":false,
                "size_bytes":null,"modified_unix":null,
            });
            if !config::safe_relative_path(relative) { return result; }
            let path = root.join(relative);
            let Some(root_real) = &canonical_root else { return result };
            let Ok(link_info) = std::fs::symlink_metadata(&path) else { return result };
            if link_info.file_type().is_symlink() { return result; }
            let Ok(real_path) = path.canonicalize() else { return result };
            if !real_path.starts_with(root_real) { return result; }
            result["exists"] = json!(true);
            result["size_bytes"] = if link_info.is_file() { json!(link_info.len()) } else { Value::Null };
            result["is_directory"] = json!(link_info.is_dir());
            if let Ok(modified) = link_info.modified() {
                if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                    result["modified_unix"] = json!(duration.as_secs());
                }
            }
            result
        }).collect()
}

/// Explicitly requested bounded candidate analysis of a registered Python
/// entrypoint. The response contains only document paths/metadata; no source
/// code, workbook contents, or TXT contents are sent to the browser.
async fn task_file_candidates(
    State(ctx): State<WebContext>,
    Path((project_name, task_name)): Path<(String, String)>,
) -> Json<Value> {
    let cfg = ctx.state.config.read().await;
    let Some(project) = cfg.project.iter().find(|p| p.name == project_name &&
        p.tasks.contains(&task_name)) else {
        return Json(json!({"ok":false,"error":"project/task not found"}));
    };
    let Some(task) = cfg.task.iter().find(|t| t.name == task_name) else {
        return Json(json!({"ok":false,"error":"task not found"}));
    };
    Json(json!({"ok":true,"files":crate::core::script_files::discover(project, task),
        "partial":true,"message":"Static Python string candidates only; dynamic paths may be missing."}))
}

#[derive(Deserialize)]
struct OpenFileRequest { path: String, confirmed: bool }

/// Open an individually-confirmed local document using the existing Windows
/// association. Recompute the server-side candidate list to prevent the API
/// from becoming an arbitrary file launcher; never accept executables/UNC.
async fn open_task_file(
    State(ctx): State<WebContext>,
    Path((project_name, task_name)): Path<(String, String)>,
    Json(req): Json<OpenFileRequest>,
) -> Json<Value> {
    if !req.confirmed {
        return Json(json!({"ok":false,"error":"explicit confirmation required"}));
    }
    let cfg = ctx.state.config.read().await;
    let local = cfg.server.bind.parse::<std::net::SocketAddr>()
        .is_ok_and(|addr| addr.ip().is_loopback());
    if !local {
        return Json(json!({"ok":false,"error":"file opening is only available on loopback server"}));
    }
    let Some(project) = cfg.project.iter().find(|p| p.name == project_name &&
        p.tasks.contains(&task_name)) else {
        return Json(json!({"ok":false,"error":"project/task not found"}));
    };
    let Some(task) = cfg.task.iter().find(|t| t.name == task_name) else {
        return Json(json!({"ok":false,"error":"task not found"}));
    };
    let files = crate::core::script_files::discover(project, task);
    let allowed = files.iter().any(|f| f.resolved_path == req.path && f.exists);
    drop(cfg);
    if !allowed {
        return Json(json!({"ok":false,"error":"document must be a known, existing candidate"}));
    }
    #[cfg(windows)]
    {
        // The -Command string is constant; user path is passed as an env var,
        // not interpolated into a PowerShell expression (no command injection).
        match std::process::Command::new("powershell.exe")
            .args(["-NoProfile","-NonInteractive","-Command",
                "Start-Process -FilePath $env:PERCHFLOW_OPEN_DOCUMENT"])
            .env("PERCHFLOW_OPEN_DOCUMENT", &req.path)
            .creation_flags(std::os::windows::process::CommandExt::CREATE_NO_WINDOW)
            .spawn() {
                Ok(_) => Json(json!({"ok":true,"requested":true,
                    "warning":"The document opens in the PerchFlow process desktop session."})),
                Err(e) => Json(json!({"ok":false,"error":format!("open request failed: {e}")})),
            }
    }
    #[cfg(not(windows))]
    {
        Json(json!({"ok":false,"error":"local file opening is Windows-only"}))
    }
}

async fn create(State(ctx): State<WebContext>, Json(project): Json<ProjectDef>) -> Json<Value> {
    let mut cfg = ctx.state.config.write().await;
    if cfg.project.iter().any(|p| p.name == project.name) {
        return Json(json!({"ok":false,"error":"project name exists"}));
    }
    let mut next = cfg.clone();
    next.project.push(project);
    persist_checked(&mut cfg, next, &ctx)
}

async fn update(
    State(ctx): State<WebContext>,
    Path(name): Path<String>,
    Json(project): Json<ProjectDef>,
) -> Json<Value> {
    let mut cfg = ctx.state.config.write().await;
    let Some(index) = cfg.project.iter().position(|p| p.name == name) else {
        return Json(json!({"ok":false,"error":"not found"}));
    };
    if name != project.name && cfg.project.iter().any(|p| p.name == project.name) {
        return Json(json!({"ok":false,"error":"project name exists"}));
    }
    let mut next = cfg.clone();
    next.project[index] = project;
    persist_checked(&mut cfg, next, &ctx)
}

async fn remove(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<Value> {
    let mut cfg = ctx.state.config.write().await;
    if !cfg.project.iter().any(|p| p.name == name) {
        return Json(json!({"ok":false,"error":"not found"}));
    }
    let mut next = cfg.clone();
    next.project.retain(|p| p.name != name);
    persist_checked(&mut cfg, next, &ctx)
}

fn persist_checked(
    current: &mut config::model::Config,
    next: config::model::Config,
    ctx: &WebContext,
) -> Json<Value> {
    if let Err(e) = config::validate(&next) {
        return Json(json!({"ok":false,"error":e.to_string()}));
    }
    if let Err(e) = writer::save(&ctx.state.config_path, &next) {
        return Json(json!({"ok":false,"error":e.to_string()}));
    }
    *current = next;
    Json(json!({"ok":true}))
}


#[derive(Debug, Deserialize)]
struct PythonTaskRequest {
    name: String,
    python: String,
    script: String,
    #[serde(default)]
    args: Vec<String>,
    schedule: String,
    #[serde(default)]
    timeout_secs: u64,
}

/// Create a *disabled* scheduled Python task belonging to the given project.
/// This only writes metadata to perch.toml, it NEVER launches the command.
async fn add_python_task(
    State(ctx): State<WebContext>,
    Path(name): Path<String>,
    Json(body): Json<PythonTaskRequest>,
) -> Json<Value> {
    if body.name.trim().is_empty() || body.name.len() > 100 {
        return Json(json!({"ok":false,"error":"task name must be 1-100 characters"}));
    }
    if body.python.trim().is_empty() || !FsPath::new(&body.python).is_absolute() {
        return Json(json!({"ok":false,"error":"Python interpreter needs an absolute path"}));
    }
    if !config::safe_relative_path(&body.script) || !body.script.to_ascii_lowercase().ends_with(".py") {
        return Json(json!({"ok":false,"error":"script must be a relative .py path inside the project"}));
    }
    if body.args.len() > 100 || body.args.iter().any(|arg| arg.len() > 2048) {
        return Json(json!({"ok":false,"error":"too many or oversized arguments"}));
    }
    if cron::Schedule::from_str(&body.schedule).is_err() {
        return Json(json!({"ok":false,"error":"invalid cron schedule"}));
    }

    let mut config = ctx.state.config.write().await;
    let Some(index) = config.project.iter().position(|p| p.name == name) else {
        return Json(json!({"ok":false,"error":"project not found"}));
    };
    if config.task.iter().any(|task| task.name == body.name) {
        return Json(json!({"ok":false,"error":"task name already exists"}));
    }
    let project_root = FsPath::new(&config.project[index].root);
    let script = project_root.join(&body.script);
    // Validate the exact script, including symlinked parent directories. No reading or execution.
    let Some(canonical_root) = project_root.canonicalize().ok() else {
        return Json(json!({"ok":false,"error":"project directory is not currently available"}));
    };
    let Ok(info) = std::fs::symlink_metadata(&script) else {
        return Json(json!({"ok":false,"error":"Python script does not exist"}));
    };
    if !info.is_file() || info.file_type().is_symlink() ||
        !script.canonicalize().is_ok_and(|real| real.starts_with(&canonical_root)) {
        return Json(json!({"ok":false,"error":"script must be a regular file inside project directory"}));
    }
    let Some(script_string) = script.to_str().map(str::to_owned) else {
        return Json(json!({"ok":false,"error":"script path cannot be encoded"}));
    };
    let mut args = vec![script_string];
    args.extend(body.args);
    let mut next = config.clone();
    next.task.push(TaskDef {
        name: body.name.clone(), cmd: body.python, args,
        cwd: Some(next.project[index].root.clone()), schedule: body.schedule,
        timeout_secs: body.timeout_secs, enabled: false,
    });
    next.project[index].tasks.push(body.name.clone());
    if !next.project[index].inputs.contains(&body.script) {
        next.project[index].inputs.push(body.script);
    }
    if let Err(err) = config::validate(&next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    if let Err(err) = writer::save(&ctx.state.config_path, &next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    *config = next;
    drop(config);
    ctx.state.tasks.write().await.insert(
        body.name.clone(), std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::core::state::TaskRuntime {
                last_run:None, last_exit_code:None, status:"idle".into(),
                started_at:None, finished_at:None, run_id:None,
                pid:None, cpu_percent:0.0, mem_bytes:0,
            }
        )),
    );
    // No scheduler.reload() here because the task is deliberately disabled.
    Json(json!({"ok":true,"task":body.name,"enabled":false}))
}


/// Read-only current readiness check. Does not launch the task or open file bodies.
/// Uses the same bearer-token protection as other /api routes.
async fn preflight_status(State(ctx): State<WebContext>, Path(name): Path<String>) -> Json<Value> {
    let config = ctx.state.config.read().await;
    let Some(project) = config.project.iter().find(|p| p.name == name) else {
        return Json(json!({"ok":false,"error":"project not found"}));
    };
    let report = crate::core::task_preflight::check(project);
    Json(json!({"ok":true,"preflight":report}))
}


#[derive(Debug, Deserialize)]
struct ScheduleToggleRequest {
    enabled: bool,
}

/// Project-scoped toggle of an existing task's cron loop. Does not execute or
/// cancel the task, change its command, or alter its input/output files.
/// Manual runs use the existing scheduler and remain independent of this flag.
async fn set_project_task_schedule(
    State(ctx): State<WebContext>,
    Path((project_name, task_name)): Path<(String, String)>,
    Json(request): Json<ScheduleToggleRequest>,
) -> Json<Value> {
    let mut current = ctx.state.config.write().await;
    let belongs = current.project.iter().any(|project| {
        project.name == project_name && project.tasks.iter().any(|t| t == &task_name)
    });
    if !belongs {
        return Json(json!({"ok":false,"error":"task does not belong to project"}));
    }
    let Some(task_index) = current.task.iter().position(|t| t.name == task_name) else {
        return Json(json!({"ok":false,"error":"task not found"}));
    };
    if current.task[task_index].enabled == request.enabled {
        return Json(json!({"ok":true,"enabled":request.enabled,"unchanged":true}));
    }
    let mut next = current.clone();
    next.task[task_index].enabled = request.enabled;
    // A schedule toggle must preserve all other task configuration.
    if let Err(err) = config::validate(&next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    if let Err(err) = writer::save(&ctx.state.config_path, &next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    *current = next;
    drop(current);
    // Disabling Cron doesn't cancel an already-running manual/scheduled run.
    ctx.scheduler.reload().await;
    Json(json!({"ok":true,"enabled":request.enabled}))
}
