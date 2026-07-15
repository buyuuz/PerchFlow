use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartConfig {
    pub policy: RestartPolicy,
    pub backoff_secs: u64,
    pub backoff_max_secs: u64,
    pub max_retries: u32,
}

impl Default for RestartConfig {
    fn default() -> Self {
        Self { policy: RestartPolicy::OnFailure, backoff_secs: 2, backoff_max_secs: 60, max_retries: 0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDef {
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub restart: RestartConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    pub schedule: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub process: Vec<ProcessDef>,
    #[serde(default)]
    pub task: Vec<TaskDef>,
}
