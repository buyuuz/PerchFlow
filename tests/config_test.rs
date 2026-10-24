use perch::config::{self, model::RestartPolicy};
use std::io::Write;

fn write_tmp(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[test]
fn loads_valid_config() {
    let f = write_tmp(r#"
[server]
bind = "127.0.0.1:9000"
token = "abc"

[[process]]
name = "app"
cmd = "ping"
args = ["127.0.0.1"]
autostart = true

[[task]]
name = "backup"
cmd = "backup.bat"
schedule = "0 0 3 * * *"
"#);
    let cfg = config::load(f.path()).unwrap();
    assert_eq!(cfg.process.len(), 1);
    assert_eq!(cfg.process[0].name, "app");
    assert_eq!(cfg.process[0].restart.policy, RestartPolicy::OnFailure);
    assert_eq!(cfg.task[0].schedule, "0 0 3 * * *");
}

#[test]
fn rejects_duplicate_process_names() {
    let f = write_tmp(r#"
[server]
bind = "127.0.0.1:9000"
token = "abc"
[[process]]
name = "dup"
cmd = "a"
[[process]]
name = "dup"
cmd = "b"
"#);
    assert!(config::load(f.path()).is_err());
}

#[test]
fn rejects_invalid_cron() {
    let f = write_tmp(r#"
[server]
bind = "127.0.0.1:9000"
token = "abc"
[[task]]
name = "t"
cmd = "a"
schedule = "not a cron"
"#);
    assert!(config::load(f.path()).is_err());
}

#[test]
fn save_then_load_roundtrips() {
    use perch::config::model::*;
    use std::collections::HashMap;
    let cfg = Config {
        server: ServerConfig { bind: "127.0.0.1:9000".into(), token: "tok".into(), lang: "en".into() },
        process: vec![ProcessDef {
            alias_zh: Some("应用管理".into()),
            name: "app".into(), cmd: "ping".into(), args: vec!["localhost".into()],
            cwd: None, env: HashMap::new(), web_url: None, autostart: true, restart: RestartConfig::default(),
        }],
        task: vec![],
        project: vec![],
    };
    let f = tempfile::NamedTempFile::new().unwrap();
    let path = f.path().to_path_buf();
    drop(f);
    perch::config::writer::save(&path, &cfg).unwrap();
    let loaded = perch::config::load(&path).unwrap();
    assert_eq!(loaded.process[0].name, "app");
    assert_eq!(loaded.process[0].alias_zh.as_deref(), Some("应用管理"));
    assert_eq!(loaded.server.token, "tok");
}

#[test]
fn manual_only_task_roundtrips_without_cron() {
    let f = write_tmp(r#"
[server]
bind = "127.0.0.1:9000"
token = "abc"
[[task]]
name = "manual_only"
cmd = "cmd"
enabled = false
"#);
    let cfg = config::load(f.path()).unwrap();
    assert!(cfg.task[0].schedule.is_empty());
    let output = tempfile::tempdir().unwrap();
    let saved = output.path().join("perch.toml");
    perch::config::writer::save(&saved, &cfg).unwrap();
    let reread = config::load(&saved).unwrap();
    assert!(reread.task[0].schedule.is_empty());
    assert!(!reread.task[0].enabled);
}

#[test]
fn rejects_enabled_manual_only_task() {
    let f = write_tmp(r#"
[server]
bind = "127.0.0.1:9000"
token = "abc"
[[task]]
name = "cannot_schedule_without_cron"
cmd = "cmd"
enabled = true
"#);
    assert!(config::load(f.path()).is_err());
}
