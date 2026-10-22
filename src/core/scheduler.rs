use crate::config::model::TaskDef;
use crate::core::process::build_command;
use crate::core::task_log_paths;
use crate::core::state::SharedState;
use crate::error::{PerchError, Result};
use chrono::{Local, Utc};
use cron::Schedule;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_util::sync::CancellationToken;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::{info, warn};

struct RunningTask {
    run_id: String,
    cancel: CancellationToken,
}

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub struct Scheduler {
    state: SharedState,
    stops: Mutex<HashMap<String, Arc<Notify>>>,
    running: Mutex<HashMap<String, RunningTask>>,
    history_lock: Mutex<()>,
    lifecycle: Mutex<()>,
}

impl Scheduler {
    pub fn new(state: SharedState) -> Arc<Self> {
        Arc::new(Self { state, stops: Mutex::new(HashMap::new()), running: Mutex::new(HashMap::new()), history_lock: Mutex::new(()), lifecycle: Mutex::new(()) })
    }

    /// Shared guard serializes claiming a run against deletion of its definition.
    pub async fn lock_lifecycle(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.lifecycle.lock().await
    }

    async fn find_task(&self, name: &str) -> Result<TaskDef> {
        let cfg = self.state.config.read().await;
        cfg.task.iter().find(|t| t.name == name).cloned()
            .ok_or_else(|| PerchError::Process(format!("unknown task: {name}")))
    }

    /// Boot or re-sync schedule loops from the current config snapshot.
    /// Stops existing loops, then spawns one loop per enabled task.
    /// Concurrent reloads are serialized by `stops`.
    pub async fn start(self: &Arc<Self>) {
        self.reload().await;
    }

    /// Rebuild schedule loops from live config (create / update / remove / enable).
    pub async fn reload(self: &Arc<Self>) {
        let mut stops = self.stops.lock().await;
        for n in stops.values() {
            // A schedule may currently be inside run_once (not awaiting
            // notified()). notify_waiters would lose that stop signal;
            // notify_one preserves a permit for its next wait.
            n.notify_one();
        }
        stops.clear();

        let tasks: Vec<TaskDef> = {
            let cfg = self.state.config.read().await;
            cfg.task.iter().filter(|t| t.enabled && !t.schedule.trim().is_empty()).cloned().collect()
        };
        for t in tasks {
            let notify = Arc::new(Notify::new());
            stops.insert(t.name.clone(), notify.clone());
            let this = self.clone();
            tokio::spawn(async move { this.schedule_loop(t, notify).await; });
        }
    }

    async fn schedule_loop(self: Arc<Self>, task: TaskDef, notify: Arc<Notify>) {
        let schedule = match Schedule::from_str(&task.schedule) {
            Ok(s) => s,
            Err(e) => {
                warn!("task '{}' has invalid cron '{}': {e}", task.name, task.schedule);
                return;
            }
        };
        info!("task '{}' scheduled with cron '{}'", task.name, task.schedule);
        loop {
            let next = match schedule.upcoming(Local).next() {
                Some(n) => n,
                None => {
                    warn!("task '{}' has no upcoming schedule, stopping loop", task.name);
                    return;
                }
            };
            let dur = (next - Local::now()).to_std().unwrap_or(std::time::Duration::from_secs(1));
            tokio::select! {
                _ = tokio::time::sleep(dur) => {
                    info!("task '{}' triggered", task.name);
                    match self.run_once(&task.name).await {
                        Ok(()) => info!("task '{}' finished", task.name),
                        Err(e) => warn!("task '{}' failed: {e}", task.name),
                    }
                }
                _ = notify.notified() => break,
            }
        }
    }

    async fn claim(&self, name: &str) -> Result<(TaskDef, String, CancellationToken)> {
        let task = self.find_task(name).await?;
        let run_id = format!("{}-{}-{}", Utc::now().timestamp_millis(), std::process::id(), RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed));
        let cancel = CancellationToken::new();
        {
            let mut running = self.running.lock().await;
            if running.contains_key(name) {
                return Err(PerchError::Process(format!("task {name} is already running")));
            }
            running.insert(name.to_owned(), RunningTask { run_id: run_id.clone(), cancel: cancel.clone() });
        }
        if let Some(rt) = self.state.tasks.read().await.get(name).cloned() {
            let mut r = rt.write().await;
            r.status = "running".into();
            r.started_at = Some(Utc::now());
            r.finished_at = None;
            r.run_id = Some(run_id.clone());
            r.pid = None;
            r.cpu_percent = 0.0;
            r.mem_bytes = 0;
        }
        Ok((task, run_id, cancel))
    }

    pub async fn run_once(&self, name: &str) -> Result<()> {
        let (task, run_id, cancel) = {
            let _guard = self.lifecycle.lock().await;
            self.claim(name).await?
        };
        self.execute(&task, &run_id, cancel).await
    }

    pub async fn start_now(self: &Arc<Self>, name: &str) -> Result<String> {
        let (task, run_id, cancel) = {
            let _guard = self.lifecycle.lock().await;
            self.claim(name).await?
        };
        let scheduler = Arc::clone(self);
        let id = run_id.clone();
        tokio::spawn(async move {
            if let Err(e) = scheduler.execute(&task, &id, cancel).await {
                warn!("manual task '{}' failed: {e}", task.name);
            }
        });
        Ok(run_id)
    }

    pub async fn cancel_task(&self, name: &str) -> Result<String> {
        let running = self.running.lock().await;
        let task = running.get(name).ok_or_else(|| PerchError::Process(format!("task {name} is not running")))?;
        task.cancel.cancel();
        Ok(task.run_id.clone())
    }

    pub async fn is_running(&self, name: &str) -> bool {
        self.running.lock().await.contains_key(name)
    }

    async fn execute(&self, task: &TaskDef, run_id: &str, cancel: CancellationToken) -> Result<()> {
        let name = &task.name;
        let started = Utc::now();
        let rt = self.state.tasks.read().await.get(name).cloned();
        let log_dir = self.state.config_path.parent().unwrap_or(std::path::Path::new(".")).join("task-logs");
        let project = {
            let cfg = self.state.config.read().await;
            cfg.project.iter().find(|p| p.tasks.iter().any(|name| name == &task.name)).cloned()
        };
        let preflight = project.as_ref().map(crate::core::task_preflight::check);
        let blocked = preflight.as_ref().is_some_and(|report| !report.ready);
        let files_before = if blocked { None } else { project.as_ref().map(crate::core::task_files::snapshot) };
        self.append_history(&log_dir, &serde_json::json!({"task":name,"run_id":run_id,"started_at":started,"status":"running","event":"started"})).await;
        let result = if blocked {
            // Never start the Python/command process when required files are missing.
            Err(PerchError::Process("preflight blocked: required input unavailable".into()))
        } else if cancel.is_cancelled() {
            Err(PerchError::Process(format!("task {name} cancelled")))
        } else {
            self.execute_child(task, &log_dir, run_id, cancel).await
        };
        let mut files_recorded = false;
        let mut output_summary: Option<serde_json::Value> = None;
        if let (Some(project), Some(before)) = (project.as_ref(), files_before) {
            let after = crate::core::task_files::snapshot(project);
            let changes = crate::core::task_files::diff(name, run_id, project, before, after);
            let output_changes: Vec<_> = changes.changes.iter().filter(|change| change.role == "output").collect();
            output_summary = Some(serde_json::json!({
                "created": output_changes.iter().filter(|c| c.change == "created").count(),
                "modified": output_changes.iter().filter(|c| c.change == "modified").count(),
                "removed": output_changes.iter().filter(|c| c.change == "removed").count(),
                "incomplete_scan": changes.incomplete_scan,
                "observed_paths": changes.scanned_paths,
            }));
            match crate::core::task_files::store(&log_dir, &changes) {
                Ok(()) => files_recorded = true,
                Err(e) => {
                    output_summary = None;
                    warn!("task '{}' file metadata snapshot failed: {e}", name);
                },
            }
        }
        let finished = Utc::now();
        let (status, code) = if blocked {
            ("blocked", None)
        } else { match &result {
            Ok(code) if *code == 0 => ("success", Some(*code)),
            Ok(code) => ("failed", Some(*code)),
            Err(e) if e.to_string().contains("timed out") => ("timeout", None),
            Err(e) if e.to_string().contains("cancelled") => ("cancelled", None),
            Err(_) => ("failed", None),
        }};
        // Publish the terminal status only after its durable history entry is written.
        // Otherwise the UI can show "success" before /history is ready.
        let duration_ms = (finished - started).num_milliseconds().max(0);
        let record = serde_json::json!({
            "task":name,"run_id":run_id,"started_at":started,"finished_at":finished,
            "duration_ms":duration_ms,"status":status,"event":"finished",
            "exit_code":code,"files_recorded":files_recorded,
            "output_summary":output_summary,"preflight":preflight,
            "error":result.as_ref().err().map(|e| e.to_string())
        });
        self.append_history(&log_dir, &record).await;
        // Keep running-map + TaskRuntime terminal transition under one lock.
        // An older finished run must never remove a subsequent run's claim.
        let mut running = self.running.lock().await;
        if let Some(rt) = rt {
            let mut r = rt.write().await;
            if r.run_id.as_deref() == Some(run_id) {
                r.status = status.into();
                r.last_run = Some(started);
                r.last_exit_code = code;
                r.finished_at = Some(finished);
                r.pid = None;
                r.cpu_percent = 0.0;
                r.mem_bytes = 0;
            }
        }
        if running.get(name).is_some_and(|task| task.run_id == run_id) {
            running.remove(name);
        }
        drop(running);
        match result {
            Ok(0) => Ok(()),
            Ok(code) => Err(PerchError::Process(format!("task {name} exited with code {code}"))),
            Err(e) => Err(e),
        }
    }

    async fn prune_task_logs(&self, log_dir: &std::path::Path, task_name: &str, current: &str) -> std::io::Result<()> {
        let config = self.state.config.read().await;
        let legacy_ok = task_log_paths::legacy_unique(
            task_name, config.task.iter().map(|task| task.name.as_str()));
        drop(config);
        prune_run_logs(log_dir, task_name, current, legacy_ok)
    }

    async fn append_history(&self, log_dir: &std::path::Path, record: &serde_json::Value) {
        let _lock = self.history_lock.lock().await;
        if let Err(e) = std::fs::create_dir_all(log_dir) {
            warn!("task history directory failed: {e}");
            return;
        }
        use std::io::Write;
        let file = std::fs::OpenOptions::new().create(true).append(true).open(log_dir.join("history.jsonl"));
        match file {
            Ok(mut f) => {
                if let Err(e) = writeln!(f, "{record}") {
                    warn!("task history write failed: {e}");
                }
            }
            Err(e) => warn!("task history open failed: {e}"),
        }
    }

    async fn execute_child(&self, task: &TaskDef, log_dir: &std::path::Path, run_id: &str, cancel: CancellationToken) -> Result<i32> {
        std::fs::create_dir_all(log_dir).map_err(|e| PerchError::Process(format!("create task log directory: {e}")))?;
        let log_file = task_log_paths::log_path(log_dir, &task.name, run_id);
        std::fs::create_dir_all(log_file.parent().expect("task hash directory"))
            .map_err(|e| PerchError::Process(format!("create task hash log directory: {e}")))?;
        let stdout = std::fs::File::create(&log_file).map_err(|e| PerchError::Process(format!("task log file: {e}")))?;
        let stderr = stdout.try_clone().map_err(|e| PerchError::Process(format!("task log clone: {e}")))?;
        let mut cmd = build_command_for_task(task);
        // Python normally block-buffers stdout when redirected to a file.
        // Unbuffer only Python children so run logs can be followed live,
        // without changing the user's script, CLI flags or environment globally.
        let executable_name = std::path::Path::new(&task.cmd)
            .file_name().and_then(|name| name.to_str()).unwrap_or("")
            .to_ascii_lowercase();
        if matches!(executable_name.as_str(), "python.exe" | "python3.exe" | "pythonw.exe" | "python" | "python3") {
            cmd.env("PYTHONUNBUFFERED", "1");
        }
        cmd.stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr)).kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| PerchError::Process(format!("spawn task {}: {e}", task.name)))?;
        if let Some(rt) = self.state.tasks.read().await.get(&task.name).cloned() {
            let mut guard = rt.write().await;
            if guard.run_id.as_deref() == Some(run_id) { guard.pid = child.id(); }
        }
        if let Err(e) = self.prune_task_logs(log_dir, &task.name, run_id).await {
            warn!("log retention for task '{}' failed: {e}", task.name);
        }
        #[cfg(windows)]
        let _job = {
            match crate::core::jobobj::JobObject::create() {
                Ok(job) => match child.raw_handle() {
                    Some(handle) => match job.assign(handle) {
                        Ok(()) => Some(job),
                        Err(e) => { warn!("task '{}' job assignment failed: {e}", task.name); None }
                    },
                    None => None,
                },
                Err(e) => { warn!("task '{}' job creation failed: {e}", task.name); None }
            }
        };
        let result: Result<i32> = tokio::select! {
            status = child.wait() => status
                .map(|s| s.code().unwrap_or(-1))
                .map_err(|e| PerchError::Process(format!("wait task {}: {e}", task.name))),
            _ = cancel.cancelled() => Err(PerchError::Process(format!("task {} cancelled", task.name))),
            _ = tokio::time::sleep(std::time::Duration::from_secs(task.timeout_secs)), if task.timeout_secs > 0 =>
                Err(PerchError::Process(format!("task {} timed out after {} seconds", task.name, task.timeout_secs))),
        };
        if result.is_err() {
            #[cfg(windows)]
            {
                if let Some(job) = &_job {
                    job.terminate();
                } else if let Some(pid) = child.id() {
                    // Job assignment can fail if the process is already in another Windows job.
                    // Fall back to killing its whole process tree rather than only cmd.exe.
                    let _ = tokio::process::Command::new("taskkill")
                        .arg("/PID").arg(pid.to_string()).args(["/T", "/F"])
                        .output().await;
                }
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        result
    }

    pub async fn shutdown(&self) {
        let mut stops = self.stops.lock().await;
        for n in stops.values() {
            n.notify_waiters();
        }
        stops.clear();
        for task in self.running.lock().await.values() {
            task.cancel.cancel();
        }
    }
}

/// Retain only the most recent three log bodies for this task across both
/// current hash, prior v2 hash, and unique legacy files. Never rewrite history.jsonl
/// or delete another task's data when legacy stems collide.
fn prune_run_logs(log_dir: &std::path::Path, task_name: &str, current: &str, legacy_ok: bool) -> std::io::Result<()> {
    let mut files: Vec<(u64, String, std::path::PathBuf)> = Vec::new();
    // Include old v2 hash logs in the same three-run retention policy;
    // only current hash directory receives new writes.
    for directory in [
        task_log_paths::task_dir(log_dir,task_name),
        task_log_paths::prior_v2_dir(log_dir,task_name),
    ] {
        if !directory.is_dir() { continue; }
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() { continue; }
            let filename=entry.file_name();
            let Some(run)=filename.to_str().and_then(|s|s.strip_suffix(".log")) else { continue };
            if !is_valid_run_id(run) { continue; }
            let timestamp=run.split('-').next().unwrap_or("0").parse::<u64>().unwrap_or(0);
            files.push((timestamp,run.to_owned(),entry.path()));
        }
    }
    if legacy_ok {
        let prefix = format!("{}-", task_log_paths::legacy_stem(task_name));
        for entry in std::fs::read_dir(log_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() { continue; }
            let filename = entry.file_name();
            let Some(run) = filename.to_str()
                .and_then(|s| s.strip_prefix(&prefix))
                .and_then(|s| s.strip_suffix(".log")) else { continue };
            if !is_valid_run_id(run) { continue; }
            let timestamp = run.split('-').next().unwrap_or("0").parse::<u64>().unwrap_or(0);
            files.push((timestamp, run.to_owned(), entry.path()));
        }
    }
    // Always retain the active run despite clock skew; other runs sorted by ID.
    files.sort_by(|a,b| (b.1 == current).cmp(&(a.1 == current))
        .then_with(|| b.0.cmp(&a.0)).then_with(|| b.1.cmp(&a.1)));
    for (_, _, path) in files.iter().skip(3) {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn is_valid_run_id(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 3 && parts.iter().all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_digit()))
}

fn build_command_for_task(task: &TaskDef) -> tokio::process::Command {
    let def = crate::config::model::ProcessDef {
        alias_zh: None,
        name: task.name.clone(), cmd: task.cmd.clone(), args: task.args.clone(),
        cwd: task.cwd.clone(), env: Default::default(), autostart: false,
        restart: Default::default(),
    };
    let mut cmd = build_command(&def);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::*;
    use crate::core::state::AppState;
    use std::path::PathBuf;

    fn sample_task(name: &str, enabled: bool) -> TaskDef {
        TaskDef {
            name: name.into(),
            cmd: "cmd".into(),
            args: vec!["/c".into(), "exit 0".into()],
            cwd: None,
            schedule: "0 0 0 * * *".into(),
            timeout_secs: 3600,
            enabled,
            history_since: None,
        }
    }

    fn state_with(tasks: Vec<TaskDef>) -> SharedState {
        let cfg = Config {
            server: ServerConfig { bind: "127.0.0.1:0".into(), token: "t".into(), lang: "en".into() },
            process: vec![],
            task: tasks,
            project: vec![],
        };
        AppState::new(cfg, PathBuf::from("perch.toml"))
    }

    #[test]
    fn retention_of_three_logs_is_scoped_to_one_task() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        std::fs::write(folder.join("history.jsonl"), b"run summaries remain").unwrap();
        std::fs::write(folder.join("Alpha-1-3-1.log"), "one").unwrap();
        std::fs::write(folder.join("Alpha-2-3-2.log"), "two").unwrap();
        std::fs::write(folder.join("Alpha-3-3-3.log"), "three").unwrap();
        std::fs::write(folder.join("Alpha-4-3-4.log"), "four").unwrap();
        std::fs::write(folder.join("Beta-1-3-1.log"), "untouched").unwrap();
        std::fs::write(folder.join("Alpha-not-a-run.log"), "not a run").unwrap();
        prune_run_logs(folder, "Alpha", "4-3-4", true).unwrap();
        assert!(!folder.join("Alpha-1-3-1.log").exists());
        for name in ["Alpha-2-3-2.log","Alpha-3-3-3.log","Alpha-4-3-4.log",
                     "Beta-1-3-1.log","Alpha-not-a-run.log","history.jsonl"] {
            assert!(folder.join(name).is_file(), "must keep {name}");
        }
    }

    #[test]
    fn v2_and_legacy_retention_is_safe_for_unicode_names() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path();
        let a = "自动发布";
        let b = "每日巡检";
        assert_eq!(task_log_paths::legacy_stem(a), task_log_paths::legacy_stem(b));
        let a_dir = task_log_paths::task_dir(root, a);
        let b_dir = task_log_paths::task_dir(root, b);
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::create_dir_all(&b_dir).unwrap();
        for run in ["1-3-1", "2-3-2", "3-3-3", "4-3-4"] {
            std::fs::write(task_log_paths::log_path(root, a, run), "A").unwrap();
        }
        std::fs::write(task_log_paths::log_path(root, b, "2-3-7"), "B").unwrap();
        std::fs::write(task_log_paths::legacy_log_path(root, a, "1-3-9"), "UNKNOWN").unwrap();
        prune_run_logs(root, a, "4-3-4", false).unwrap();
        assert!(!task_log_paths::log_path(root,a,"1-3-1").exists());
        assert!(task_log_paths::log_path(root,a,"4-3-4").exists());
        assert!(task_log_paths::log_path(root,b,"2-3-7").exists());
        // The ambiguous old prefix is never pruned or read as either task.
        assert!(task_log_paths::legacy_log_path(root,a,"1-3-9").exists());
        assert!(task_log_paths::resolve_log_path(root,a,"1-3-9",false).is_none());
        assert!(task_log_paths::resolve_log_path(root,b,"1-3-9",false).is_none());
    }

    #[test]
    fn retention_spans_unique_legacy_and_v2_formats() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path();
        std::fs::write(root.join("legacy-1-3-1.log"), "one").unwrap();
        std::fs::write(root.join("legacy-2-3-2.log"), "two").unwrap();
        let target = task_log_paths::task_dir(root,"legacy");
        std::fs::create_dir_all(target).unwrap();
        std::fs::write(task_log_paths::log_path(root,"legacy","3-3-3"),"three").unwrap();
        std::fs::write(task_log_paths::log_path(root,"legacy","4-3-4"),"four").unwrap();
        prune_run_logs(root,"legacy","4-3-4",true).unwrap();
        assert!(!root.join("legacy-1-3-1.log").exists());
        assert!(root.join("legacy-2-3-2.log").exists());
        assert!(task_log_paths::log_path(root,"legacy","3-3-3").exists());
        assert!(task_log_paths::log_path(root,"legacy","4-3-4").exists());
    }

    #[test]
    fn retention_includes_prior_v2_and_direct_hash_paths() {
        let tmp=tempfile::tempdir().unwrap();
        let root=tmp.path();
        let name="每日巡检";
        let old=task_log_paths::prior_v2_dir(root,name);
        let new=task_log_paths::task_dir(root,name);
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        for id in ["1-2-1","2-2-2"] {
            std::fs::write(old.join(format!("{id}.log")),"old").unwrap();
        }
        for id in ["3-2-3","4-2-4"] {
            std::fs::write(new.join(format!("{id}.log")),"new").unwrap();
        }
        prune_run_logs(root,name,"4-2-4",false).unwrap();
        assert!(!old.join("1-2-1.log").exists());
        assert!(old.join("2-2-2.log").exists());
        assert!(new.join("3-2-3.log").exists());
        assert!(new.join("4-2-4.log").exists());
    }

    #[test]
    fn new_run_id_is_not_mistaken_for_old_or_invalid_filename() {
        assert!(is_valid_run_id("1780000000000-2350-1"));
        for id in ["../1-2-3","123-a-2","12-3","12-3-4.log","4-3-3-5",""] {
            assert!(!is_valid_run_id(id), "reject {id}");
        }
    }

    #[tokio::test]
    async fn run_once_records_exit_code() {
        let state = state_with(vec![sample_task("t", true)]);
        let sched = Scheduler::new(state.clone());
        sched.run_once("t").await.unwrap();
        let rt = state.tasks.read().await.get("t").cloned().unwrap();
        let r = rt.read().await;
        assert_eq!(r.last_exit_code, Some(0));
        assert!(r.last_run.is_some());
    }

    #[tokio::test]
    async fn reload_syncs_enabled_tasks() {
        let state = state_with(vec![sample_task("a", true), sample_task("b", false)]);
        let sched = Scheduler::new(state.clone());
        sched.start().await;
        {
            let stops = sched.stops.lock().await;
            assert!(stops.contains_key("a"));
            assert!(!stops.contains_key("b"));
        }

        // enable b, disable a
        {
            let mut cfg = state.config.write().await;
            for t in &mut cfg.task {
                t.enabled = t.name == "b";
            }
        }
        sched.reload().await;
        {
            let stops = sched.stops.lock().await;
            assert!(!stops.contains_key("a"));
            assert!(stops.contains_key("b"));
        }

        // create c via config and reload
        {
            let mut cfg = state.config.write().await;
            cfg.task.push(sample_task("c", true));
        }
        sched.reload().await;
        {
            let stops = sched.stops.lock().await;
            assert!(!stops.contains_key("a"));
            assert!(stops.contains_key("b"));
            assert!(stops.contains_key("c"));
        }
    }
}
