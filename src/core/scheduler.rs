use crate::config::model::TaskDef;
use crate::core::process::build_command;
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
}

impl Scheduler {
    pub fn new(state: SharedState) -> Arc<Self> {
        Arc::new(Self { state, stops: Mutex::new(HashMap::new()), running: Mutex::new(HashMap::new()), history_lock: Mutex::new(()) })
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
            n.notify_waiters();
        }
        stops.clear();

        let tasks: Vec<TaskDef> = {
            let cfg = self.state.config.read().await;
            cfg.task.iter().filter(|t| t.enabled).cloned().collect()
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
        }
        Ok((task, run_id, cancel))
    }

    pub async fn run_once(&self, name: &str) -> Result<()> {
        let (task, run_id, cancel) = self.claim(name).await?;
        let result = self.execute(&task, &run_id, cancel).await;
        self.running.lock().await.remove(name);
        result
    }

    pub async fn start_now(self: &Arc<Self>, name: &str) -> Result<String> {
        let (task, run_id, cancel) = self.claim(name).await?;
        let scheduler = Arc::clone(self);
        let id = run_id.clone();
        tokio::spawn(async move {
            if let Err(e) = scheduler.execute(&task, &id, cancel).await {
                warn!("manual task '{}' failed: {e}", task.name);
            }
            scheduler.running.lock().await.remove(&task.name);
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
        self.append_history(&log_dir, &serde_json::json!({"task":name,"run_id":run_id,"started_at":started,"status":"running","event":"started"})).await;
        let result = self.execute_child(task, &log_dir, run_id, cancel).await;
        let finished = Utc::now();
        let (status, code) = match &result {
            Ok(code) if *code == 0 => ("success", Some(*code)),
            Ok(code) => ("failed", Some(*code)),
            Err(e) if e.to_string().contains("timed out") => ("timeout", None),
            Err(e) if e.to_string().contains("cancelled") => ("cancelled", None),
            Err(_) => ("failed", None),
        };
        // Publish the terminal status only after its durable history entry is written.
        // Otherwise the UI can show "success" before /history is ready.
        let record = serde_json::json!({"task":name,"run_id":run_id,"started_at":started,"finished_at":finished,"status":status,"event":"finished","exit_code":code,"error":result.as_ref().err().map(|e| e.to_string())});
        self.append_history(&log_dir, &record).await;
        if let Some(rt) = rt {
            let mut r = rt.write().await;
            r.status = status.into();
            r.last_run = Some(started);
            r.last_exit_code = code;
            r.finished_at = Some(finished);
        }
        match result {
            Ok(0) => Ok(()),
            Ok(code) => Err(PerchError::Process(format!("task {name} exited with code {code}"))),
            Err(e) => Err(e),
        }
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
        let safe_name: String = task.name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
        let log_file = log_dir.join(format!("{safe_name}-{run_id}.log"));
        let stdout = std::fs::File::create(&log_file).map_err(|e| PerchError::Process(format!("task log file: {e}")))?;
        let stderr = stdout.try_clone().map_err(|e| PerchError::Process(format!("task log clone: {e}")))?;
        let mut cmd = build_command_for_task(task);
        cmd.stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr)).kill_on_drop(true);
        let mut child = cmd.spawn().map_err(|e| PerchError::Process(format!("spawn task {}: {e}", task.name)))?;
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
