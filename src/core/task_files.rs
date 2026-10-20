//! Bounded, metadata-only per-run file tracking.
//!
//! Explicit input/output file paths and optional project-relative output
//! directories are monitored. Directory scanning never opens file contents,
//! follows links outside the project root, or walks deeper than two levels.
use crate::config::{self, model::ProjectDef};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    time::UNIX_EPOCH,
};

pub const MAX_TRACKED_PATHS: usize = 128;
pub const MAX_OUTPUT_DIRS: usize = 8;
pub const MAX_DISCOVERED_FILES: usize = 512;
pub const MAX_VISITED_DIRS: usize = 64;
/// The root output directory and one child directory are visited.
pub const MAX_SCAN_DEPTH: usize = 2;
const MAX_SIDECAR_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileState {
    pub role: String,
    pub path: String,
    pub exists: bool,
    pub is_directory: bool,
    pub size_bytes: Option<u64>,
    pub modified_ns: Option<u128>,
    #[serde(default)]
    pub discovered: bool,
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
    #[serde(default)]
    pub incomplete_scan: bool,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub entries: Vec<FileState>,
    /// On unreadable directories or limits, do not infer that unseen files
    /// were created/removed. Never claim an exhaustive inventory.
    pub complete: bool,
}

fn missing(role: &str, relative: &str, discovered: bool) -> FileState {
    FileState {
        role: role.into(), path: relative.into(), exists: false,
        is_directory: false, size_bytes: None, modified_ns: None, discovered,
    }
}

fn file_state(
    root: &Path, canonical_root: Option<&Path>, role: &str, relative: &str,
    discovered: bool,
) -> FileState {
    let mut result = missing(role, relative, discovered);
    if !config::safe_relative_path(relative) { return result; }
    let Some(canonical_root) = canonical_root else { return result };
    let candidate = root.join(relative);
    let Ok(info) = std::fs::symlink_metadata(&candidate) else { return result };
    if info.file_type().is_symlink() { return result; }
    let Ok(real) = candidate.canonicalize() else { return result };
    if !real.starts_with(canonical_root) { return result; }
    result.exists = true;
    result.is_directory = info.is_dir();
    result.size_bytes = info.is_file().then_some(info.len());
    result.modified_ns = info.modified().ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos());
    result
}

/// Read only metadata for explicitly configured paths, plus at most 512
/// regular files found under at most eight registered output directories.
/// No contents, hashes, or unknown project directories are scanned.
pub fn snapshot(project: &ProjectDef) -> Snapshot {
    let root = Path::new(&project.root);
    let canonical_root = root.canonicalize().ok();
    let mut entries: BTreeMap<(String, String), FileState> = BTreeMap::new();
    let mut complete = true;
    let registered = project.inputs.iter().map(|name| ("input", name))
        .chain(project.outputs.iter().map(|name| ("output", name)));
    for (role, relative) in registered.take(MAX_TRACKED_PATHS) {
        let state = file_state(root, canonical_root.as_deref(), role, relative, false);
        entries.insert((role.into(), relative.clone()), state);
    }
    if project.inputs.len().saturating_add(project.outputs.len()) > MAX_TRACKED_PATHS {
        complete = false;
    }
    if project.output_dirs.len() > MAX_OUTPUT_DIRS {
        complete = false;
    }
    let mut discovered = 0usize;
    let mut directories_visited = 0usize;

    'all_directories: for relative_dir in project.output_dirs.iter().take(MAX_OUTPUT_DIRS) {
        let root_entry = file_state(root, canonical_root.as_deref(), "output_dir", relative_dir, false);
        entries.insert(("output_dir".into(), relative_dir.clone()), root_entry.clone());
        if !root_entry.exists { continue; } // output directory may be created by task
        if !root_entry.is_directory { complete = false; continue; }
        let mut pending = vec![(root.join(relative_dir), 0usize)];
        while let Some((directory, depth)) = pending.pop() {
            if directories_visited >= MAX_VISITED_DIRS {
                complete = false;
                break 'all_directories;
            }
            directories_visited += 1;
            let read_dir = match std::fs::read_dir(&directory) {
                Ok(iter) => iter,
                Err(_) => { complete = false; continue; }
            };
            for result in read_dir {
                let child = match result {
                    Ok(child) => child,
                    Err(_) => { complete = false; continue; }
                };
                let child_path = child.path();
                let relative_path = match child_path.strip_prefix(root)
                    .ok().and_then(|s| s.to_str()) {
                    Some(s) if config::safe_relative_path(s) => s.to_owned(),
                    _ => { complete = false; continue; }
                };
                let info = match std::fs::symlink_metadata(&child_path) {
                    Ok(info) => info,
                    Err(_) => { complete = false; continue; }
                };
                if info.file_type().is_symlink() { continue; }
                let real = match child_path.canonicalize() {
                    Ok(path) => path,
                    Err(_) => { complete = false; continue; }
                };
                if !canonical_root.as_ref().is_some_and(|base| real.starts_with(base)) {
                    // A reparse point/junction must not take us outside root.
                    continue;
                }
                if info.is_dir() {
                    if depth + 1 < MAX_SCAN_DEPTH {
                        pending.push((child_path, depth + 1));
                    }
                    continue;
                }
                if !info.is_file() { continue; }
                if discovered >= MAX_DISCOVERED_FILES {
                    complete = false;
                    break 'all_directories;
                }
                discovered += 1;
                let state = file_state(root, canonical_root.as_deref(), "output", &relative_path, true);
                if state.exists {
                    // An explicit reference takes precedence over auto-discovered metadata.
                    entries.entry(("output".into(), relative_path)).or_insert(state);
                }
            }
        }
    }
    Snapshot { entries: entries.into_values().collect(), complete }
}

pub fn diff(task: &str, run_id: &str, project: &ProjectDef, before: Snapshot, after: Snapshot) -> RunFileChanges {
    let scanned_paths = before.entries.len().max(after.entries.len());
    let incomplete_scan = !before.complete || !after.complete;
    let before_map: BTreeMap<(String, String), FileState> = before.entries.into_iter()
        .map(|f| ((f.role.clone(), f.path.clone()), f)).collect();
    let after_map: BTreeMap<(String, String), FileState> = after.entries.into_iter()
        .map(|f| ((f.role.clone(), f.path.clone()), f)).collect();
    let mut changes = Vec::new();
    for (key, old) in &before_map {
        let new = after_map.get(key);
        let kind = match new {
            Some(new) if !old.exists && new.exists => Some("created"),
            Some(new) if old.exists && !new.exists && after.complete => Some("removed"),
            Some(new) if old.exists && new.exists &&
                (old.size_bytes != new.size_bytes || old.modified_ns != new.modified_ns ||
                 old.is_directory != new.is_directory) => Some("modified"),
            None if old.exists && after.complete => Some("removed"),
            _ => None,
        };
        if let Some(kind) = kind {
            let replacement = new.cloned()
                .unwrap_or_else(|| missing(&old.role, &old.path, old.discovered));
            changes.push(FileChange {
                role: old.role.clone(), path: old.path.clone(), change: kind.into(),
                before: old.clone(), after: replacement,
            });
        }
    }
    for (key, new) in &after_map {
        if !before_map.contains_key(key) && new.exists && before.complete {
            changes.push(FileChange {
                role: new.role.clone(), path: new.path.clone(), change: "created".into(),
                before: missing(&new.role, &new.path, new.discovered),
                after: new.clone(),
            });
        }
    }
    changes.sort_by(|a, b| a.path.cmp(&b.path).then(a.role.cmp(&b.role)));
    RunFileChanges {
        task: task.into(), run_id: run_id.into(), project: project.name.clone(),
        scanned_paths, changes, finished_at: chrono::Utc::now().to_rfc3339(), incomplete_scan,
    }
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
    if data.len() as u64 > MAX_SIDECAR_BYTES {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "file metadata exceeds limit"));
    }
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
    let mut file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_SIDECAR_BYTES {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "file metadata too large"));
    }
    use std::io::Read;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
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

    fn project(dir: &Path, inputs: &[&str], outputs: &[&str], output_dirs: &[&str]) -> ProjectDef {
        ProjectDef {
            name: "mock".into(), root: dir.to_string_lossy().into_owned(),
            tasks: vec![], inputs: inputs.iter().map(|s| (*s).into()).collect(),
            outputs: outputs.iter().map(|s| (*s).into()).collect(),
            output_dirs: output_dirs.iter().map(|s| (*s).into()).collect(),
        }
    }

    #[test]
    fn explicit_paths_still_track_created_modified_removed_without_body() {
        let dir = tempdir().unwrap();
        let p = project(dir.path(), &[], &["out.txt"], &[]);
        let before = snapshot(&p);
        std::fs::write(dir.path().join("out.txt"), "SECRET_DUMMY_TEST_TOKEN").unwrap();
        let changes = diff("mock_task", "123-456-7", &p, before, snapshot(&p));
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
    fn discovers_files_in_registered_output_directory_and_nested_subdirectory() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("artifacts");
        std::fs::create_dir(&output).unwrap();
        let p = project(dir.path(), &[], &[], &["artifacts"]);
        let before = snapshot(&p);
        std::fs::write(output.join("result.xlsx"), "DUMMY_PRIVATE_WORKBOOK").unwrap();
        std::fs::create_dir(output.join("images")).unwrap();
        std::fs::write(output.join("images").join("photo.png"), "MOCK_IMAGE").unwrap();
        std::fs::create_dir(output.join("images").join("deep")).unwrap();
        std::fs::write(output.join("images").join("deep").join("ignored.jpg"), "IGNORED").unwrap();
        let changes = diff("job", "run-1", &p, before, snapshot(&p));
        assert!(changes.changes.iter().any(|x| x.path.ends_with("result.xlsx") && x.change == "created" && x.after.discovered));
        assert!(changes.changes.iter().any(|x| x.path.ends_with("photo.png") && x.change == "created"));
        assert!(!changes.changes.iter().any(|x| x.path.contains("ignored.jpg")));
        assert!(!serde_json::to_string(&changes).unwrap().contains("DUMMY_PRIVATE_WORKBOOK"));
        let before = snapshot(&p);
        std::fs::write(output.join("result.xlsx"), "LONGER_DUMMY_WORKBOOK").unwrap();
        let changes = diff("job", "run-2", &p, before, snapshot(&p));
        assert!(changes.changes.iter().any(|x| x.path.ends_with("result.xlsx") && x.change == "modified"));
        let before = snapshot(&p);
        std::fs::remove_file(output.join("result.xlsx")).unwrap();
        let changes = diff("job", "run-3", &p, before, snapshot(&p));
        assert!(changes.changes.iter().any(|x| x.path.ends_with("result.xlsx") && x.change == "removed"));
    }

    #[test]
    fn newly_created_output_directory_is_detected() {
        let dir = tempdir().unwrap();
        let p = project(dir.path(), &[], &[], &["new_results"]);
        let before = snapshot(&p);
        std::fs::create_dir(dir.path().join("new_results")).unwrap();
        std::fs::write(dir.path().join("new_results").join("result.txt"), "FAKE_RESULT").unwrap();
        let changes = diff("job", "run-1", &p, before, snapshot(&p));
        assert!(changes.changes.iter().any(|x| x.path.ends_with("result.txt") && x.change == "created"));
    }

    #[test]
    fn respects_entry_limits_and_does_not_infer_false_deletions_on_truncated_scan() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("many");
        std::fs::create_dir(&output).unwrap();
        let p = project(dir.path(), &[], &[], &["many"]);
        for i in 0..(MAX_DISCOVERED_FILES + 10) {
            std::fs::write(output.join(format!("file-{i:04}.txt")), "MOCK_ONLY").unwrap();
        }
        let before = snapshot(&p);
        let after = snapshot(&p);
        assert!(!before.complete && !after.complete);
        let changes = diff("job", "run-1", &p, before, after);
        assert!(changes.incomplete_scan);
        assert!(changes.changes.iter().all(|x| x.change != "removed" && x.change != "created"));
    }

    #[test]
    fn excludes_symlinks_and_traversal() {
        let dir = tempdir().unwrap();
        let p = project(dir.path(), &["../outside"], &[], &[]);
        assert!(!snapshot(&p).entries[0].exists);
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            let out = dir.path().join("outside");
            std::fs::create_dir(&out).unwrap();
            let sensitive = out.join("secret.txt");
            std::fs::write(&sensitive, "DUMMY_PRIVATE").unwrap();
            let tracked = dir.path().join("outputs");
            std::fs::create_dir(&tracked).unwrap();
            if symlink_file(&sensitive, tracked.join("secret-link.txt")).is_ok() {
                let p = project(dir.path(), &[], &[], &["outputs"]);
                assert!(!snapshot(&p).entries.iter().any(|s| s.path.contains("secret-link.txt")));
            }
        }
    }
}
