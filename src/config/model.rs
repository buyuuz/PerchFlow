use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub token: String,
    #[serde(default = "default_lang")]
    pub lang: String,
}

fn default_lang() -> String { "en".into() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self { bind: "127.0.0.1:9000".into(), token: "change-me".into(), lang: "en".into() }
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
    /// Optional Chinese display alias; `name` remains the process identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias_zh: Option<String>,
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Optional manual web UI override; None enables automatic local port discovery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_url: Option<String>,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub restart: RestartConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    /// Human-readable task name; identity remains unique in `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub name: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub schedule: String,
    #[serde(default = "default_task_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// A generation boundary for new tasks, preventing a reused name from
    /// inheriting run history and logs belonging to a retired task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_since: Option<chrono::DateTime<chrono::Utc>>,
}

fn default_true() -> bool { true }
fn default_task_timeout() -> u64 { 0 }

/// A project groups existing scheduled tasks and references files in their original directory.
/// No files are copied, opened, or executed when a project is saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDef {
    pub name: String,
    pub root: String,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Project-relative output directories to scan for file metadata after each run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_dirs: Vec<String>,
    /// Opt-in validation of declared input files before starting any project task.
    /// False preserves legacy behavior for existing projects and Python scripts.
    #[serde(default)]
    pub preflight_inputs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    #[serde(default)]
    pub process: Vec<ProcessDef>,
    #[serde(default)]
    pub task: Vec<TaskDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project: Vec<ProjectDef>,
}

impl Default for Config {
    fn default() -> Self {
        Self { server: ServerConfig::default(), process: Vec::new(), task: Vec::new(), project: Vec::new() }
    }
}

/// 首次运行时(exe 同目录无 perch.toml)生成的默认配置:含一个 demo 进程,
/// 让用户登录后能立刻看到 running 状态的卡片,可用 UI 删除或编辑。
#[cfg(test)]
mod alias_tests {
    use super::*;

    #[test]
    fn legacy_process_configs_without_alias_still_load() {
        let original = default_demo_config();
        let saved = toml::to_string(&original).expect("serialize demo config");
        assert!(!saved.contains("alias_zh"), "None should not be serialized");
        let parsed: Config = toml::from_str(&saved).expect("parse legacy config");
        assert!(parsed.process[0].alias_zh.is_none());
        assert_eq!(parsed.process[0].name, "demo");
    }

    #[test]
    fn chinese_alias_survives_toml_roundtrip_without_changing_process_name() {
        let mut cfg = default_demo_config();
        cfg.process[0].alias_zh = Some("邮箱管理中心".into());
        let saved = toml::to_string(&cfg).expect("serialize alias");
        assert!(saved.contains("alias_zh"));
        let restored: Config = toml::from_str(&saved).expect("parse alias");
        assert_eq!(restored.process[0].name, "demo");
        assert_eq!(restored.process[0].alias_zh.as_deref(), Some("邮箱管理中心"));
    }
}

pub fn default_demo_config() -> Config {
    Config {
        server: ServerConfig::default(),
        process: vec![ProcessDef {
            name: "demo".into(),
            alias_zh: None,
            cmd: "cmd".into(),
            args: vec!["/c".into(), "ping -n 100 127.0.0.1".into()],
            cwd: None,
            env: std::collections::HashMap::new(),
            web_url: None,
            web_url: None,
            autostart: true,
            restart: RestartConfig::default(),
        }],
        task: Vec::new(),
        project: Vec::new(),
    }
}
