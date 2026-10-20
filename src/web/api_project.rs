//! Project registry over the existing task scheduler. No file content access.
//! A project is metadata: an original working directory, existing task links,
//! and relative input/output file references. Deleting a project never deletes
//! task definitions, Python scripts, files or run logs.
use crate::config::{self, model::ProjectDef, writer};
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
