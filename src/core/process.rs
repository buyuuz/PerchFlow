use crate::config::model::ProcessDef;
use crate::core::state::{ProcRuntime, ProcStatus};
use crate::error::{PerchError, Result};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

pub fn build_command(def: &ProcessDef) -> Command {
    let mut cmd = Command::new(&def.cmd);
    cmd.args(&def.args);
    if let Some(cwd) = &def.cwd {
        cmd.current_dir(cwd);
    }
    for (k, v) in &def.env {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

pub async fn spawn_process(
    def: &ProcessDef,
    runtime: Arc<RwLock<ProcRuntime>>,
) -> Result<Child> {
    let mut cmd = build_command(def);
    let mut child = cmd.spawn().map_err(|e| PerchError::Process(format!("spawn {}: {e}", def.name)))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let pid = child.id();

    {
        let mut rt = runtime.write().await;
        rt.status = ProcStatus::Running;
        rt.pid = pid;
        rt.started_at = Some(Instant::now());
    }

    if let Some(out) = stdout {
        spawn_reader(out, runtime.clone());
    }
    if let Some(err) = stderr {
        spawn_reader(err, runtime.clone());
    }
    Ok(child)
}

fn spawn_reader<R>(reader: R, runtime: Arc<RwLock<ProcRuntime>>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            runtime.write().await.logs.push(line);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{ProcessDef, RestartConfig};
    use crate::core::state::{ProcRuntime, ProcStatus};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn echo_def() -> ProcessDef {
        // Windows: cmd /c echo hello
        ProcessDef {
            name: "echo".into(),
            cmd: "cmd".into(),
            args: vec!["/c".into(), "echo hello".into()],
            cwd: None,
            env: HashMap::new(),
            autostart: false,
            restart: RestartConfig::default(),
        }
    }

    #[tokio::test]
    async fn spawn_captures_stdout() {
        let rt = Arc::new(RwLock::new(ProcRuntime {
            status: ProcStatus::Stopped, pid: None, started_at: None,
            restart_count: 0, cpu_percent: 0.0, mem_bytes: 0,
            logs: crate::core::ringbuf::RingBuffer::new(100),
        }));
        let mut child = spawn_process(&echo_def(), rt.clone()).await.unwrap();
        let _ = child.wait().await;
        // 给读取 task 一点时间
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let logs = rt.read().await.logs.recent(10);
        assert!(logs.iter().any(|l| l.contains("hello")), "logs={logs:?}");
    }
}
