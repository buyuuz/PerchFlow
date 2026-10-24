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
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn routes() -> Router<WebContext> {
    Router::new()
        .route("/projects", get(list).post(create))
        .route("/projects/:name", get(detail).put(update).delete(remove))
        .route("/projects/:name/python-task", axum::routing::post(add_python_task))
        .route("/projects/:name/tasks/:task", axum::routing::delete(delete_project_task))
        .route("/projects/:name/tasks/:task/files", get(task_file_candidates))
        .route("/projects/:name/tasks/:task/settings", axum::routing::put(update_script_settings))
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
#[serde(deny_unknown_fields)]
struct ScriptSettingsRequest {
    cmd: String,
    script: String,
    cwd: String,
    #[serde(default)]
    args: Vec<String>,
    timeout_secs: u64,
}

/// Edit only a registered Python script's execution fields. Cron configuration,
/// task identity, project membership, and run history must remain unchanged.
async fn update_script_settings(
    State(ctx): State<WebContext>,
    Path((project_name, task_name)): Path<(String, String)>,
    Json(req): Json<ScriptSettingsRequest>,
) -> Json<Value> {
    if req.cmd.trim().is_empty() || !FsPath::new(&req.cmd).is_absolute() {
        return Json(json!({"ok":false,"error":"Python interpreter needs an absolute path"}));
    }
    if req.cwd.trim().is_empty() || !FsPath::new(&req.cwd).is_absolute() {
        return Json(json!({"ok":false,"error":"working directory needs an absolute path"}));
    }
    if req.args.len() > 100 || req.args.iter().any(|arg| arg.len() > 2048) {
        return Json(json!({"ok":false,"error":"too many or oversized arguments"}));
    }
    if ctx.scheduler.is_running(&task_name).await {
        return Json(json!({"ok":false,"error":"task is running; stop it before editing"}));
    }
    let mut config = ctx.state.config.write().await;
    let Some(project) = config.project.iter().find(|p|
        p.name == project_name && p.tasks.contains(&task_name)) else {
        return Json(json!({"ok":false,"error":"project/task not found"}));
    };
    let root = FsPath::new(&project.root);
    let Some(canonical_root) = root.canonicalize().ok() else {
        return Json(json!({"ok":false,"error":"project directory is not currently available"}));
    };
    let requested = FsPath::new(req.script.trim());
    if !req.script.to_ascii_lowercase().ends_with(".py") ||
        (!requested.is_absolute() && !config::safe_relative_path(&req.script)) {
        return Json(json!({"ok":false,"error":"script must be a .py file inside the project"}));
    }
    let script = if requested.is_absolute() { requested.to_path_buf() } else { root.join(requested) };
    let Ok(info) = std::fs::symlink_metadata(&script) else {
        return Json(json!({"ok":false,"error":"Python script does not exist"}));
    };
    if !info.is_file() || info.file_type().is_symlink() ||
        !script.canonicalize().is_ok_and(|real| real.starts_with(&canonical_root)) {
        return Json(json!({"ok":false,"error":"script must be a regular file inside project directory"}));
    }
    if !FsPath::new(&req.cwd).is_dir() {
        return Json(json!({"ok":false,"error":"working directory does not exist"}));
    }
    let Some(script_path) = script.to_str().map(str::to_owned) else {
        return Json(json!({"ok":false,"error":"script path cannot be encoded"}));
    };
    let mut next = config.clone();
    let Some(task) = next.task.iter_mut().find(|t| t.name == task_name) else {
        return Json(json!({"ok":false,"error":"task not found"}));
    };
    // Preserve schedule/enabled/identity in the same locked transaction.
    task.cmd = req.cmd;
    task.cwd = Some(req.cwd);
    task.args = std::iter::once(script_path).chain(req.args).collect();
    task.timeout_secs = req.timeout_secs;
    if let Err(err) = config::validate(&next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    if let Err(err) = writer::save(&ctx.state.config_path, &next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    *config = next;
    drop(config);
    ctx.scheduler.reload().await;
    Json(json!({"ok":true}))
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
            .creation_flags(0x0800_0000)
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
    // Optional: a project script is manual-only until the user creates a schedule.
    // Legacy clients may still send a schedule; ignore it for new project scripts.
    #[serde(default)]
    schedule: Option<String>,
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
    // No Cron is provisioned by creating a Python script. Scheduling is a
    // separate, explicit action; any legacy body.schedule is ignored.
    let _legacy_schedule = &body.schedule;

    let mut config = ctx.state.config.write().await;
    let Some(index) = config.project.iter().position(|p| p.name == name) else {
        return Json(json!({"ok":false,"error":"project not found"}));
    };
    // Names are unique within a project, but different projects may share
    // the same visible label. All execution/runtime/log operations still use
    // a global, collision-proof internal task key.
    let visible_name=body.name.trim().to_string();
    if config.project[index].tasks.iter().any(|key| config.task.iter().any(|task|
        &task.name==key && task.display_name.as_deref().unwrap_or(&task.name)==visible_name)) {
        return Json(json!({"ok":false,"error":"script name already exists in this project"}));
    }
    let mut internal_name=visible_name.clone();
    if config.task.iter().any(|task|task.name==internal_name) {
        use sha2::{Digest,Sha256};
        let digest=Sha256::digest(format!("{}\0{}\0{}",name,visible_name,chrono::Utc::now().timestamp_micros()).as_bytes());
        let hash=format!("{digest:x}");
        internal_name=format!("pf_{}", &hash[..24]);
        let mut salt=0u32;
        while config.task.iter().any(|task|task.name==internal_name) {
            salt+=1; internal_name=format!("pf_{}_{}", &hash[..18],salt);
        }
    }
    let display_name=(internal_name!=visible_name).then_some(visible_name);
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
        display_name: None,
        display_name, name: internal_name.clone(), cmd: body.python, args,
        cwd: Some(next.project[index].root.clone()), schedule: String::new(),
        timeout_secs: body.timeout_secs, enabled: false,
        history_since: Some(chrono::Utc::now()),
    });
    next.project[index].tasks.push(internal_name.clone());
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
        internal_name.clone(), std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::core::state::TaskRuntime {
                last_run:None, last_exit_code:None, status:"idle".into(),
                started_at:None, finished_at:None, run_id:None,
                pid:None, cpu_percent:0.0, mem_bytes:0,
            }
        )),
    );
    // No scheduler.reload() here because the task is deliberately disabled.
    Json(json!({"ok":true,"task":internal_name,"enabled":false}))
}



/// Removes a project-owned *task definition*, not the Python source, project
/// inputs/outputs, external documents, or retained execution log files.
async fn delete_project_task(
    State(ctx): State<WebContext>, Path((project_name, task_name)): Path<(String, String)>,
) -> Json<Value> {
    // lock order: lifecycle -> scheduler running -> config. start_now/run_once
    // acquire lifecycle across claim, closing the check-then-delete race.
    let _lifecycle=ctx.scheduler.lock_lifecycle().await;
    if ctx.scheduler.is_running(&task_name).await {
        return Json(json!({"ok":false,"error":"task is running; stop it before deleting"}));
    }
    let mut current=ctx.state.config.write().await;
    let Some(project_index)=current.project.iter().position(|p|
        p.name==project_name && p.tasks.iter().any(|name| name==&task_name)) else {
        return Json(json!({"ok":false,"error":"project/task not found"}));
    };
    if current.task.iter().all(|t|t.name!=task_name) {
        return Json(json!({"ok":false,"error":"task not found"}));
    }
    let mut next=current.clone();
    let script=current.task.iter().find(|t|t.name==task_name)
        .and_then(|t|t.args.iter().find(|a|a.to_ascii_lowercase().ends_with(".py")))
        .cloned();
    next.task.retain(|t|t.name!=task_name);
    next.project[project_index].tasks.retain(|t|t!=&task_name);
    // Python script inputs were added automatically by add_python_task.
    // Remove an exclusive stale registration, but preserve shared references.
    if let Some(script)=script {
        let root=next.project[project_index].root.clone();
        let norm=|value: &str| value.replace('/', "\\").to_ascii_lowercase();
        let old_script=norm(&script);
        let referenced_by_other=next.project[project_index].tasks.iter().any(|other| {
            next.task.iter().find(|t| &t.name==other)
                .is_some_and(|t|t.args.iter().any(|arg|norm(arg)==old_script))
        });
        if !referenced_by_other {
            next.project[project_index].inputs.retain(|input| {
                norm(&format!("{}\\{}",root.trim_end_matches(['\\','/']),input))!=old_script
            });
        }
    }
    if let Err(err)=config::validate(&next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    if let Err(err)=writer::save(&ctx.state.config_path,&next) {
        return Json(json!({"ok":false,"error":err.to_string()}));
    }
    *current=next;
    drop(current);
    ctx.state.tasks.write().await.remove(&task_name);
    ctx.scheduler.reload().await;
    Json(json!({"ok":true,"project":project_name,"task":task_name,
        "source_files_deleted":false}))
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
    if current.task[task_index].schedule.is_empty() {
        return Json(json!({"ok":false,"error":"no schedule configured; add a schedule first"}));
    }
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
