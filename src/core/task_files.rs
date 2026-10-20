//! Metadata-only snapshots of explicitly registered project files.
//! The snapshot never opens file bodies, follows symlinks outside the project
//! root, copies credentials, or recursively scans arbitrary directories.
use crate::config::{self, model::ProjectDef};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileState {
    pub role: String,
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub size_bytes: Option<u64>,
    pub modified_ns: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub role: String,
    pub path: String,
    pub change: String,
    pub before: FileState,
    pub after: FileState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunFileChanges {
    pub task: String,
    pub run_id: String,
    pub project: String,
    pub scanned_paths: usize,
    pub changes: Vec<FileChange>,
    pub finished_at: String,
}

fn state(root: &Path, role: &str, relative: &str) -> FileState {
    let mut entry = FileState {
        role: role.to_owned(), path: relative.to_owned(), exists: false,
        is_directory: false, size_bytes: None, modified_ns: None,
    };
    if !config::safe_relative_path(relative) { return entry; }
    let Some(root_real) = root.canonicalize().ok() else { return entry };
    let candidate = root.join(relative);
    // Final symlinks are rejected; canonicalize also prevents symlinked
    // intermediate directories from escaping the declared project root.
    let Ok(info) = std::fs::symlink_metadata(&candidate) else { return entry };
    if info.file_type().is_symlink() { return entry; }
    let Ok(real) = candidate.canonicalize() else { return entry };
    if !real.starts_with(&root_real) { return entry; }
    entry.exists = true;
    entry.is_directory = info.is_dir();
    entry.size_bytes = info.is_file().then_some(info.len());
    entry.modified_ns = info.modified().ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos());
    entry
}

/// Bound the work performed per task. Directory contents are not scanned;
/// only explicitly registered paths (including directory metadata) are tracked.
pub const MAX_TRACKED_PATHS: usize = 128;

pub fn snapshot(project: &ProjectDef) -> Vec<FileState> {
    let root = Path::new(&project.root);
    project.inputs.iter().map(|name| ("input", name))
        .chain(project.outputs.iter().map(|name| ("output", name)))
        .take(MAX_TRACKED_PATHS)
        .map(|(role, relative)| state(root, role, relative))
        .collect()
}

pub fn diff(task: &str, run_id: &str, project: &ProjectDef, before: Vec<FileState>, after: Vec<FileState>) -> RunFileChanges {
    let scanned_paths = before.len();
    let changes = before.into_iter().zip(after).filter_map(|(old, new)| {
        let kind = match (old.exists, new.exists) {
            (false, true) => Some("created"),
            (true, false) => Some("removed"),
            (true, true) if old.size_bytes != new.size_bytes || old.modified_ns != new.modified_ns
                || old.is_directory != new.is_directory => Some("modified"),
            _ => None,
        }?;
        Some(FileChange { role: old.role.clone(), path: old.path.clone(),
            change: kind.into(), before: old, after: new })
    }).collect();
    RunFileChanges { task: task.to_owned(), run_id: run_id.to_owned(), project: project.name.clone(),
        scanned_paths, changes, finished_at: chrono::Utc::now().to_rfc3339() }
}

pub fn store(log_dir: &Path, changes: &RunFileChanges) -> std::io::Result<()> {
    std::fs::create_dir_all(log_dir)?;
    let safe: String = changes.task.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let filename = format!("{safe}-{}.files.json", changes.run_id);
    let target = log_dir.join(filename);
    let data = serde_json::to_vec(changes)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    // Create new so an existing record is never silently overwritten.
    // The task completion history is written only after this operation.
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(target)?;
    use std::io::Write;
    file.write_all(&data)?;
    Ok(())
}

pub fn read(log_dir: &Path, name: &str, run_id: &str) -> std::io::Result<RunFileChanges> {
    if run_id.is_empty() || !run_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid run id"));
    }
    let safe: String = name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = log_dir.join(format!("{safe}-{run_id}.files.json"));
    // Bound sidecar size: enough for 128 file paths, no unbounded JSON read.
    let mut file = std::fs::File::open(path)?;
    use std::io::Read;
    let mut contents = Vec::new();
    file.by_ref().take(256 * 1024).read_to_end(&mut contents)?;
    let parsed: RunFileChanges = serde_json::from_slice(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if parsed.task != name || parsed.run_id != run_id {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "record mismatch"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn tracks_created_modified_removed_only_and_never_reads_contents() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let p = ProjectDef { name: "mock".into(), root,
            tasks: vec![], inputs: vec![], outputs: vec!["out.txt".into()] };
        let before = snapshot(&p);
        std::fs::write(dir.path().join("out.txt"), "SECRET_DUMMY_TEST_TOKEN").unwrap();
        let after = snapshot(&p);
        let changes = diff("mock_task", "123-456-7", &p, before, after);
        assert_eq!(changes.changes[0].change, "created");
        store(dir.path(), &changes).unwrap();
        let saved = read(dir.path(), "mock_task", "123-456-7").unwrap();
        assert_eq!(saved.changes[0].path, "out.txt");
        let raw = std::fs::read_to_string(dir.path().join("mock_task-123-456-7.files.json")).unwrap();
        assert!(!raw.contains("SECRET_DUMMY_TEST_TOKEN"));
        let before = snapshot(&p);
        std::fs::write(dir.path().join("out.txt"), "SECRET_DUMMY_TEST_TOKEN_LONGER").unwrap();
        assert_eq!(diff("mock_task", "second", &p, before, snapshot(&p)).changes[0].change, "modified");
        let before = snapshot(&p);
        std::fs::remove_file(dir.path().join("out.txt")).unwrap();
        assert_eq!(diff("mock_task", "third", &p, before, snapshot(&p)).changes[0].change, "removed");
        assert!(read(dir.path(), "mock_task", "../secret").is_err());
    }

    #[test]
    fn never_follows_symlinks_or_traversal() {
        let dir = tempdir().unwrap();
        let p = ProjectDef { name: "demo".into(), root: dir.path().to_string_lossy().into_owned(),
            tasks: vec![], inputs: vec!["../outside".into()], outputs: vec![] };
        assert!(!snapshot(&p)[0].exists);
    }
}
