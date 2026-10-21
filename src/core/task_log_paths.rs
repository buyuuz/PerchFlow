//! Collision-resistant, Windows-safe run artifact paths.
//!
//! New run artifacts use SHA-256 of the exact UTF-8 task identity directly
//! under task-logs/<hash>. Previously released v2/<hash> paths remain readable,
//! and legacy sanitized filenames remain readable when ownership is unique.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn task_key(name: &str) -> String {
    use std::fmt::Write;
    let digest = Sha256::digest(name.as_bytes());
    let mut key = String::with_capacity(64);
    for byte in digest {
        write!(&mut key, "{byte:02x}").expect("write SHA-256 hex");
    }
    key
}

pub fn task_dir(log_root: &Path, name: &str) -> PathBuf {
    log_root.join(task_key(name))
}

/// Prior hashed layout: kept for backwards-compatible reads only.
pub fn prior_v2_dir(log_root: &Path, name: &str) -> PathBuf {
    log_root.join("v2").join(task_key(name))
}

pub fn prior_v2_log_path(log_root: &Path, name: &str, run_id: &str) -> PathBuf {
    prior_v2_dir(log_root, name).join(format!("{run_id}.log"))
}

pub fn prior_v2_sidecar_path(log_root: &Path, name: &str, run_id: &str) -> PathBuf {
    prior_v2_dir(log_root, name).join(format!("{run_id}.files.json"))
}

pub fn log_path(log_root: &Path, name: &str, run_id: &str) -> PathBuf {
    task_dir(log_root, name).join(format!("{run_id}.log"))
}

pub fn sidecar_path(log_root: &Path, name: &str, run_id: &str) -> PathBuf {
    task_dir(log_root, name).join(format!("{run_id}.files.json"))
}

pub fn legacy_stem(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

pub fn legacy_log_path(log_root: &Path, name: &str, run_id: &str) -> PathBuf {
    log_root.join(format!("{}-{run_id}.log", legacy_stem(name)))
}

pub fn legacy_sidecar_path(log_root: &Path, name: &str, run_id: &str) -> PathBuf {
    log_root.join(format!("{}-{run_id}.files.json", legacy_stem(name)))
}

/// Never read or prune a legacy artifact when its old filename prefix is
/// shared by another registered task. Such files have ambiguous ownership.
pub fn legacy_unique<'a>(name: &str, names: impl IntoIterator<Item = &'a str>) -> bool {
    let stem = legacy_stem(name);
    names.into_iter().filter(|candidate| legacy_stem(candidate) == stem).count() == 1
}

pub fn resolve_log_path(log_root: &Path, name: &str, run_id: &str, legacy_ok: bool) -> Option<PathBuf> {
    let current = log_path(log_root, name, run_id);
    if current.is_file() { return Some(current); }
    let prior = prior_v2_log_path(log_root, name, run_id);
    if prior.is_file() { return Some(prior); }
    if legacy_ok {
        let old = legacy_log_path(log_root, name, run_id);
        if old.is_file() { return Some(old); }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_names_remain_distinct_with_safe_fixed_size_paths() {
        let root = Path::new("task-logs");
        let a = "自动发布";
        let b = "每日巡检";
        assert_eq!(legacy_stem(a), legacy_stem(b));
        assert_ne!(task_key(a), task_key(b));
        assert!(task_key(a).bytes().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(task_key(a).len(), 64);
        assert_eq!(task_key(&"中文".repeat(120)).len(),64);
        assert_ne!(task_key("a/b"),task_key("a\\b"));
        assert_eq!(task_dir(root,a), root.join(task_key(a)));
        assert!(!log_path(root,a,"1-2-3").to_string_lossy().contains("v2"));
        assert_eq!(prior_v2_dir(root,a),root.join("v2").join(task_key(a)));
        assert_ne!(sidecar_path(root, a, "1-2-3"),sidecar_path(root, b, "1-2-3"));
    }
    #[test]
    fn reads_prior_v2_logs_but_writes_to_direct_hash_directory() {
        let tmp=tempfile::tempdir().unwrap();
        let root=tmp.path();
        let old=prior_v2_log_path(root,"每日巡检","1-2-3");
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old,"old log").unwrap();
        assert_eq!(resolve_log_path(root,"每日巡检","1-2-3",false),Some(old));
        let fresh=log_path(root,"每日巡检","2-2-3");
        assert_eq!(fresh.parent().unwrap(),root.join(task_key("每日巡检")));
        assert!(!fresh.to_string_lossy().contains("v2"));
    }
    #[test]
    fn legacy_ambiguity_blocks_cross_task_reads() {
        assert!(!legacy_unique("自动发布", ["自动发布","每日巡检"]));
        assert!(legacy_unique("task-one", ["task-one","task-two"]));
        assert!(!legacy_unique("a/b", ["a/b","a?b"]));
    }
}
