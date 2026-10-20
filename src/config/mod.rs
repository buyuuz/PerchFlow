pub mod model;
pub mod writer;

use crate::error::{PerchError, Result};
use model::Config;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn default_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("perch.toml")))
        .unwrap_or_else(|| PathBuf::from("perch.toml"))
}

pub fn load(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| PerchError::Config(format!("read {}: {e}", path.display())))?;
    let cfg: Config = toml::from_str(&text)
        .map_err(|e| PerchError::Config(format!("parse: {e}")))?;
    validate(&cfg)?;
    Ok(cfg)
}

pub fn validate(cfg: &Config) -> Result<()> {
    let mut seen = HashSet::new();
    for p in &cfg.process {
        if p.name.trim().is_empty() {
            return Err(PerchError::Config("process name empty".into()));
        }
        if !seen.insert(("p", p.name.as_str())) {
            return Err(PerchError::Config(format!("duplicate process name: {}", p.name)));
        }
    }
    for t in &cfg.task {
        if t.name.trim().is_empty() {
            return Err(PerchError::Config("task name empty".into()));
        }
        if !seen.insert(("t", t.name.as_str())) {
            return Err(PerchError::Config(format!("duplicate task name: {}", t.name)));
        }
        cron::Schedule::from_str(&t.schedule)
            .map_err(|e| PerchError::Config(format!("bad cron for {}: {e}", t.name)))?;
    }
    // Project identities and file paths are metadata, never arbitrary filesystem routes.
    // Restrict references to relative, normal path components under the project root.
    let mut names = HashSet::new();
    let mut assigned = HashSet::new();
    for project in &cfg.project {
        if project.name.trim().is_empty() || project.name.len() > 100 {
            return Err(PerchError::Config("project name must be 1-100 characters".into()));
        }
        if !names.insert(project.name.as_str()) {
            return Err(PerchError::Config(format!("duplicate project: {}", project.name)));
        }
        let root = Path::new(&project.root);
        // Missing/offline project drives should not stop the whole PerchFlow service.
        // The read-only file-status API reports unavailable files instead.
        if !root.is_absolute() {
            return Err(PerchError::Config(format!("project {} needs an absolute directory path", project.name)));
        }
        let mut local = HashSet::new();
        for name in &project.tasks {
            if !cfg.task.iter().any(|t| &t.name == name) {
                return Err(PerchError::Config(format!("unknown task {} in project {}", name, project.name)));
            }
            if !local.insert(name.as_str()) || !assigned.insert(name.as_str()) {
                return Err(PerchError::Config(format!("task {} cannot belong to multiple projects", name)));
            }
        }
        for path in project.inputs.iter().chain(project.outputs.iter()) {
            if !safe_relative_path(path) {
                return Err(PerchError::Config(format!("unsafe file path in project {}", project.name)));
            }
        }
    }
    Ok(())
}

/// Relative paths only. Block parent, absolute, UNC and drive-prefixed paths.
pub fn safe_relative_path(raw: &str) -> bool {
    if raw.is_empty() || raw.contains('\\') && !cfg!(windows) {
        return false;
    }
    // Path components alone on Unix do not treat backslashes as separators;
    // reject traversal through either platform's separator convention.
    if raw.replace('\\', "/").split('/').any(|part| part == ".." || part.is_empty()) {
        return false;
    }
    let path = Path::new(raw);
    !raw.contains(':') && !path.is_absolute() &&
        path.components().all(|part| matches!(part, std::path::Component::Normal(_)))
}
