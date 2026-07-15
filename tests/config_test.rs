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
        server: ServerConfig { bind: "127.0.0.1:9000".into(), token: "tok".into() },
        process: vec![ProcessDef {
            name: "app".into(), cmd: "ping".into(), args: vec!["localhost".into()],
            cwd: None, env: HashMap::new(), autostart: true, restart: RestartConfig::default(),
        }],
        task: vec![],
    };
    let f = tempfile::NamedTempFile::new().unwrap();
    perch::config::writer::save(f.path(), &cfg).unwrap();
    let loaded = perch::config::load(f.path()).unwrap();
    assert_eq!(loaded.process[0].name, "app");
    assert_eq!(loaded.server.token, "tok");
}
