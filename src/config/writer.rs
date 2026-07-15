use crate::config::model::Config;
use crate::error::{PerchError, Result};
use std::path::Path;

pub fn save(path: &Path, cfg: &Config) -> Result<()> {
    let text = toml::to_string_pretty(cfg)
        .map_err(|e| PerchError::Config(format!("serialize: {e}")))?;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = dir.join(format!(".{}.tmp", std::process::id()));

    std::fs::write(&tmp_path, &text)
        .map_err(|e| PerchError::Config(format!("write tmp: {e}")))?;
    std::fs::rename(&tmp_path, path)
        .map_err(|e| PerchError::Config(format!("rename: {e}")))?;
    Ok(())
}
