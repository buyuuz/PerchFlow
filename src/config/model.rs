use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub token: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { bind: "127.0.0.1:9000".into(), token: "change-me".into() }
    }
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

impl Default for Config {
    fn default() -> Self {
        Self { server: ServerConfig::default(), process: Vec::new(), task: Vec::new() }
    }
}

/// 首次运行时(exe 同目录无 perch.toml)生成的默认配置:含一个 demo 进程,
/// 让用户登录后能立刻看到 running 状态的卡片,可用 UI 删除或编辑。
pub fn default_demo_config() -> Config {
    Config {
        server: ServerConfig::default(),
        process: vec![ProcessDef {
            name: "demo".into(),
            cmd: "cmd".into(),
            args: vec!["/c".into(), "ping -n 100 127.0.0.1".into()],
            cwd: None,
            env: std::collections::HashMap::new(),
            autostart: true,
            restart: RestartConfig::default(),
        }],
        task: Vec::new(),
    }
}
