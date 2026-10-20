use crate::config::model::Config;
use crate::core::ringbuf::RingBuffer;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
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
        let mut tasks = HashMap::new();
        for t in &config.task {
            tasks.insert(t.name.clone(), Arc::new(RwLock::new(TaskRuntime { last_run: None, last_exit_code: None, status: "idle".into(), started_at: None, finished_at: None, run_id: None })));
        }
        Arc::new(AppState {
            config: RwLock::new(config),
            procs: RwLock::new(procs),
            tasks: RwLock::new(tasks),
            config_path,
        })
    }
}
