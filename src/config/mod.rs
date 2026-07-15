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
    Ok(())
}
