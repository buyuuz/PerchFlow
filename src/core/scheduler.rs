use crate::config::model::TaskDef;
use crate::core::process::build_command;
use crate::core::state::SharedState;
use crate::error::{PerchError, Result};
use chrono::{Local, Utc};
use cron::Schedule;
use std::collections::HashMap;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tracing::{info, warn};

pub struct Scheduler {
    state: SharedState,
    stops: Mutex<HashMap<String, Arc<Notify>>>,
}

impl Scheduler {
    pub fn new(state: SharedState) -> Arc<Self> {
        Arc::new(Self { state, stops: Mutex::new(HashMap::new()) })
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
                        Ok(()) => info!("task '{}' completed", task.name),
                        Err(e) => warn!("task '{}' failed: {e}", task.name),
                    }
                }
                _ = notify.notified() => break,
            }
        }
    }

    pub async fn run_once(&self, name: &str) -> Result<()> {
        let task = self.find_task(name).await?;
        let mut cmd = build_command_for_task(&task);
        let mut child = cmd.spawn()
            .map_err(|e| PerchError::Process(format!("spawn task {name}: {e}")))?;
        let status = child.wait().await
            .map_err(|e| PerchError::Process(format!("wait task {name}: {e}")))?;
        if let Some(rt) = self.state.tasks.read().await.get(name).cloned() {
            let mut r = rt.write().await;
            r.last_run = Some(Utc::now());
            r.last_exit_code = status.code();
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut stops = self.stops.lock().await;
        for n in stops.values() {
            n.notify_waiters();
        }
        stops.clear();
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
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
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
            enabled,
        }
    }

    fn state_with(tasks: Vec<TaskDef>) -> SharedState {
        let cfg = Config {
            server: ServerConfig { bind: "127.0.0.1:0".into(), token: "t".into(), lang: "en".into() },
            process: vec![],
            task: tasks,
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
