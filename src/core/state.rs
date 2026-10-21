use crate::config::model::Config;
use crate::core::ringbuf::RingBuffer;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

const LOG_CAPACITY: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcStatus {
    Running,
    Stopped,
    Crashed,
    Restarting,
}

pub struct ProcRuntime {
    pub status: ProcStatus,
    pub pid: Option<u32>,
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub cpu_percent: f32,
    pub mem_bytes: u64,
    pub logs: RingBuffer,
}

impl ProcRuntime {
    pub fn new() -> Self {
        Self {
            status: ProcStatus::Stopped,
            pid: None,
            started_at: None,
            restart_count: 0,
            cpu_percent: 0.0,
            mem_bytes: 0,
            logs: RingBuffer::new(LOG_CAPACITY),
        }
    }
}

pub struct TaskRuntime {
    pub last_run: Option<DateTime<Utc>>,
    pub last_exit_code: Option<i32>,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub run_id: Option<String>,
    pub pid: Option<u32>,
    pub cpu_percent: f32,
    pub mem_bytes: u64,
}

pub struct AppState {
    pub config: RwLock<Config>,
    pub procs: RwLock<HashMap<String, Arc<RwLock<ProcRuntime>>>>,
    pub tasks: RwLock<HashMap<String, Arc<RwLock<TaskRuntime>>>>,
    pub config_path: PathBuf,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(config: Config, config_path: PathBuf) -> Arc<AppState> {
        let mut procs = HashMap::new();
        for p in &config.process {
            procs.insert(p.name.clone(), Arc::new(RwLock::new(ProcRuntime::new())));
        }
        // Restore the latest recorded state after restarting PerchFlow.
        // An unfinished started record is shown as "interrupted", not "running".
        let history_path = config_path.parent().unwrap_or(std::path::Path::new("."))
            .join("task-logs").join("history.jsonl");
        let mut recent: HashMap<String, serde_json::Value> = HashMap::new();
        if let Ok(file) = std::fs::File::open(history_path) {
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(name) = entry.get("task").and_then(|s| s.as_str()) {
                        recent.insert(name.to_owned(), entry);
                    }
                }
            }
        }
        let parse_time = |entry: &serde_json::Value, key: &str| -> Option<DateTime<Utc>> {
            entry.get(key).and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
        };
        let mut tasks = HashMap::new();
        for t in &config.task {
            let record = recent.get(&t.name);
            let status = record.and_then(|v| v.get("status")).and_then(|v| v.as_str()).unwrap_or("idle");
            let status = if status == "running" { "interrupted" } else { status };
            let runtime = TaskRuntime {
                last_run: record.and_then(|v| parse_time(v, "started_at")),
                last_exit_code: record.and_then(|v| v.get("exit_code")).and_then(|v| v.as_i64()).and_then(|v| i32::try_from(v).ok()),
                status: status.into(),
                started_at: record.and_then(|v| parse_time(v, "started_at")),
                finished_at: record.and_then(|v| parse_time(v, "finished_at")),
                run_id: record.and_then(|v| v.get("run_id")).and_then(|v| v.as_str()).map(str::to_owned),
                pid: None, cpu_percent: 0.0, mem_bytes: 0,
            };
            tasks.insert(t.name.clone(), Arc::new(RwLock::new(runtime)));
        }
        Arc::new(AppState {
            config: RwLock::new(config),
            procs: RwLock::new(procs),
            tasks: RwLock::new(tasks),
            config_path,
        })
    }
}

#[cfg(test)]
mod restore_tests {
    use super::*;
    use crate::config::model::TaskDef;

    #[tokio::test]
    async fn restores_interrupted_run_from_history() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("task-logs");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::write(logs.join("history.jsonl"),
            "{\"task\":\"check\",\"run_id\":\"100-1-1\",\"status\":\"running\",\"event\":\"started\",\"started_at\":\"2026-10-20T01:00:00Z\"}\n"
        ).unwrap();
        let cfg = Config {
            server: Default::default(),
            process: vec![],
            task: vec![TaskDef {
                name: "check".into(), cmd: "cmd".into(), args: vec![],
                cwd: None, schedule: "0 0 0 * * *".into(), timeout_secs: 0, enabled: false,
            }],
            project: vec![],
        };
        let app = AppState::new(cfg, dir.path().join("perch.toml"));
        let handle = app.tasks.read().await.get("check").cloned().unwrap();
        let restored = handle.read().await;
        assert_eq!(restored.status, "interrupted");
        assert_eq!(restored.run_id.as_deref(), Some("100-1-1"));
        assert_eq!(restored.last_exit_code, None);
        assert!(restored.last_run.is_some());
    }
}
