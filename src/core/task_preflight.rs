//! Read-only required input preflight. Only checks declared project inputs;
//! no file bodies or secrets are opened, copied or logged.
use crate::config::{self, model::ProjectDef};
use serde::Serialize;
use std::path::Path;

pub const MAX_REQUIRED_INPUTS: usize = 128;

#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    pub enabled: bool,
    pub checked: usize,
    pub ready: bool,
    /// Project-relative configured paths, not file contents.
    pub unavailable: Vec<String>,
    /// No sensitive filesystem error text is exposed.
    pub reason: Option<&'static str>,
}

pub fn check(project: &ProjectDef) -> Preflight {
    if !project.preflight_inputs {
        return Preflight {
            enabled: false, checked: 0, ready: true,
            unavailable: Vec::new(), reason: None,
        };
    }
    if project.inputs.len() > MAX_REQUIRED_INPUTS {
        return Preflight {
            enabled: true, checked: 0, ready: false,
            unavailable: Vec::new(), reason: Some("too_many_required_inputs"),
        };
    }
    let root = Path::new(&project.root);
    let canonical_root = root.canonicalize().ok().filter(|p| p.is_dir());
    let mut unavailable = Vec::new();
    for relative in &project.inputs {
        if !config::safe_relative_path(relative) {
            unavailable.push(relative.clone());
            continue;
        }
        let Some(root_real) = &canonical_root else {
            unavailable.push(relative.clone());
            continue;
        };
        let candidate = root.join(relative);
        let okay = std::fs::symlink_metadata(&candidate).is_ok_and(|info| {
            info.is_file() && !info.file_type().is_symlink() &&
            candidate.canonicalize().is_ok_and(|real| real.starts_with(root_real))
        });
        if !okay {
            unavailable.push(relative.clone());
        }
    }
    Preflight {
        enabled: true, checked: project.inputs.len(),
        ready: unavailable.is_empty(),
        unavailable,
        reason: if canonical_root.is_none() { Some("project_directory_unavailable") }
            else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn p(root: &std::path::Path, inputs: &[&str], enabled: bool) -> ProjectDef {
        ProjectDef {
            name: "mock".into(), root: root.to_string_lossy().into_owned(),
            tasks: vec![], inputs: inputs.iter().map(|s| s.to_string()).collect(),
            outputs: vec![], output_dirs: vec![], preflight_inputs: enabled,
        }
    }

    #[test]
    fn opt_in_and_relative_files_are_verified_without_reading_contents() {
        let dir = tempdir().unwrap();
        let project = p(dir.path(), &["accounts.txt", "config.json"], true);
        let first = check(&project);
        assert!(!first.ready);
        assert_eq!(first.unavailable, vec!["accounts.txt", "config.json"]);
        std::fs::write(dir.path().join("accounts.txt"), "SECRET_MOCK_PASSWORD").unwrap();
        let second = check(&project);
        assert!(!second.ready);
        assert_eq!(second.unavailable, vec!["config.json"]);
        std::fs::write(dir.path().join("config.json"), "SECRET_MOCK_KEY").unwrap();
        let third = check(&project);
        assert!(third.ready);
        assert_eq!(third.checked, 2);
        assert!(!serde_json::to_string(&third).unwrap().contains("SECRET_MOCK"));
    }

    #[test]
    fn legacy_opt_out_does_not_block_and_invalid_or_missing_root_blocks_opt_in() {
        let dir = tempdir().unwrap();
        let no = p(dir.path(), &["not-here.txt"], false);
        assert!(check(&no).ready);
        assert_eq!(check(&no).checked, 0);
        let invalid = p(dir.path(), &["../unrelated.txt"], true);
        assert!(!check(&invalid).ready);
        let offline = p(&dir.path().join("disconnected"), &["needed.txt"], true);
        let state = check(&offline);
        assert!(!state.ready);
        assert_eq!(state.reason, Some("project_directory_unavailable"));
    }

    #[test]
    fn too_many_inputs_fail_closed() {
        let dir = tempdir().unwrap();
        let mut project = p(dir.path(), &[], true);
        project.inputs = (0..(MAX_REQUIRED_INPUTS+1)).map(|i| format!("{i}.txt")).collect();
        let state = check(&project);
        assert!(!state.ready);
        assert_eq!(state.reason, Some("too_many_required_inputs"));
    }
}
