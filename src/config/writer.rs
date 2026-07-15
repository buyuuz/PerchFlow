use crate::config::model::Config;
use crate::error::{PerchError, Result};
use std::io::Write;
use std::path::Path;

pub fn save(path: &Path, cfg: &Config) -> Result<()> {
    let text = toml::to_string_pretty(cfg)
        .map_err(|e| PerchError::Config(format!("serialize: {e}")))?;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| PerchError::Config(format!("tmp: {e}")))?;
    tmp.write_all(text.as_bytes())
        .map_err(|e| PerchError::Config(format!("write: {e}")))?;
    tmp.persist(path)
        .map_err(|e| PerchError::Config(format!("persist: {e}")))?;
    Ok(())
}
