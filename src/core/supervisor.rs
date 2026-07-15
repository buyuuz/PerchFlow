use crate::config::model::{ProcessDef, RestartPolicy};
use crate::core::backoff::Backoff;
use crate::core::process::spawn_process;
use crate::core::state::{ProcStatus, SharedState};
use crate::error::{PerchError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct Supervisor {
    state: SharedState,
    stops: Mutex<HashMap<String, CancellationToken>>,
}

impl Supervisor {
    pub fn new(state: SharedState) -> Arc<Self> {
        Arc::new(Self { state, stops: Mutex::new(HashMap::new()) })
    }

    async fn find_def(&self, name: &str) -> Result<ProcessDef> {
        let cfg = self.state.config.read().await;
        cfg.process.iter().find(|p| p.name == name).cloned()
            .ok_or_else(|| PerchError::Process(format!("unknown process: {name}")))
    }

    pub async fn start(self: &Arc<Self>, name: &str) -> Result<()> {
        let def = self.find_def(name).await?;
        let runtime = {
            let procs = self.state.procs.read().await;
            procs.get(name).cloned()
                .ok_or_else(|| PerchError::Process(format!("no runtime: {name}")))?
        };
        let token = CancellationToken::new();
        {
            let mut stops = self.stops.lock().await;
            if stops.contains_key(name) {
                return Ok(()); // 已在运行
            }
            stops.insert(name.to_string(), token.clone());
        }
        let this = self.clone();
        let name_owned = name.to_string();
        tokio::spawn(async move {
            this.supervise_loop(def, runtime, token, name_owned).await;
        });
        Ok(())
    }

    pub async fn stop(&self, name: &str) -> Result<()> {
        let token = { self.stops.lock().await.remove(name) };
        if let Some(t) = token {
            t.cancel();
        }
        if let Some(rt) = self.state.procs.read().await.get(name).cloned() {
            let mut r = rt.write().await;
            r.status = ProcStatus::Stopped;
            r.pid = None;
        }
        Ok(())
    }

    pub async fn restart(self: &Arc<Self>, name: &str) -> Result<()> {
        self.stop(name).await?;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        self.start(name).await
    }

    pub async fn start_autostart(self: &Arc<Self>) {
        let names: Vec<String> = {
            let cfg = self.state.config.read().await;
            cfg.process.iter().filter(|p| p.autostart).map(|p| p.name.clone()).collect()
        };
        for n in names {
            let _ = self.start(&n).await;
        }
    }

    pub async fn shutdown(&self) {
        let names: Vec<String> = { self.stops.lock().await.keys().cloned().collect() };
        for n in names {
            let _ = self.stop(&n).await;
        }
    }
}

impl Supervisor {
    async fn supervise_loop(
        self: Arc<Self>,
        def: ProcessDef,
        runtime: std::sync::Arc<tokio::sync::RwLock<crate::core::state::ProcRuntime>>,
        token: CancellationToken,
        name: String,
    ) {
        let mut backoff = Backoff::new(def.restart.backoff_secs, def.restart.backoff_max_secs);
        loop {
            if token.is_cancelled() {
                // stop() 已在本次 spawn 前取消,避免再次拉起进程覆盖已写入的 Stopped 状态。
                break;
            }
            match run_child(&def, &runtime, &token).await {
                ChildOutcome::SpawnError => {
                    runtime.write().await.status = ProcStatus::Crashed;
                    break;
                }
                ChildOutcome::Cancelled => {
                    // 本循环是取消路径上唯一权威写入者,顺序晚于 spawn_process 的写入。
                    mark_stopped(&runtime).await;
                    break;
                }
                ChildOutcome::Exited(status) => {
                    if handle_exit(&def, &runtime, status).await {
                        break;
                    }
                    let delay = backoff.next_delay();
                    tokio::select! {
                        _ = tokio::time::sleep(delay) => {}
                        _ = token.cancelled() => break,
                    }
                }
            }
        }
        self.stops.lock().await.remove(&name);
    }
}

enum ChildOutcome {
    SpawnError,
    Cancelled,
    Exited(std::io::Result<std::process::ExitStatus>),
}

/// 拉起子进程并等待其退出或收到取消信号。取消检查在 spawn_process().await
/// 返回之后进行,确保写入顺序晚于 spawn_process 内部的 Running 写入。
async fn run_child(
    def: &ProcessDef,
    runtime: &std::sync::Arc<tokio::sync::RwLock<crate::core::state::ProcRuntime>>,
    token: &CancellationToken,
) -> ChildOutcome {
    let mut child = match spawn_process(def, runtime.clone()).await {
        Ok(c) => c,
        Err(_) => return ChildOutcome::SpawnError,
    };
    if token.is_cancelled() {
        let _ = child.kill().await;
        return ChildOutcome::Cancelled;
    }
    tokio::select! {
        status = child.wait() => ChildOutcome::Exited(status),
        _ = token.cancelled() => {
            let _ = child.kill().await;
            ChildOutcome::Cancelled
        }
    }
}

/// 将 runtime 标记为 Stopped 并清空 pid,用于取消路径上的权威终态写入。
async fn mark_stopped(runtime: &std::sync::Arc<tokio::sync::RwLock<crate::core::state::ProcRuntime>>) {
    let mut r = runtime.write().await;
    r.status = ProcStatus::Stopped;
    r.pid = None;
}

/// 处理一次子进程退出:更新状态与重启计数,返回 true 表示应结束监督循环(不再重启)。
async fn handle_exit(
    def: &ProcessDef,
    runtime: &std::sync::Arc<tokio::sync::RwLock<crate::core::state::ProcRuntime>>,
    status: std::io::Result<std::process::ExitStatus>,
) -> bool {
    let success = status.map(|s| s.success()).unwrap_or(false);
    if !should_restart(def.restart.policy, success) {
        runtime.write().await.status = if success { ProcStatus::Stopped } else { ProcStatus::Crashed };
        return true;
    }
    let mut r = runtime.write().await;
    r.status = ProcStatus::Restarting;
    r.restart_count += 1;
    r.pid = None;
    if def.restart.max_retries != 0 && r.restart_count > def.restart.max_retries {
        r.status = ProcStatus::Crashed;
        return true;
    }
    false
}

fn should_restart(policy: RestartPolicy, success: bool) -> bool {
    match policy {
        RestartPolicy::Always => true,
        RestartPolicy::Never => false,
        RestartPolicy::OnFailure => !success,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::*;
    use crate::core::state::AppState;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn cfg_with(def: ProcessDef) -> Config {
        Config {
            server: ServerConfig { bind: "127.0.0.1:0".into(), token: "t".into() },
            process: vec![def],
            task: vec![],
        }
    }

    #[tokio::test]
    async fn start_then_stop_marks_stopped() {
        // 长驻进程:ping -t 本机
        let def = ProcessDef {
            name: "p".into(), cmd: "cmd".into(),
            args: vec!["/c".into(), "ping -n 30 127.0.0.1".into()],
            cwd: None, env: HashMap::new(), autostart: false,
            restart: RestartConfig { policy: RestartPolicy::Never, ..Default::default() },
        };
        let state = AppState::new(cfg_with(def), PathBuf::from("perch.toml"));
        let sup = Supervisor::new(state.clone());
        sup.start("p").await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        {
            let rt = state.procs.read().await.get("p").cloned().unwrap();
            assert_eq!(rt.read().await.status, ProcStatus::Running);
        }
        sup.stop("p").await.unwrap();
        let rt = state.procs.read().await.get("p").cloned().unwrap();
        assert_eq!(rt.read().await.status, ProcStatus::Stopped);
    }

    #[tokio::test]
    async fn stop_immediately_after_start_is_race_free() {
        // 覆盖 start() 中 spawn_process().await 完成前 stop() 就到达的窗口:
        // 用 CancellationToken 取代 Notify 后,该窗口内的 cancel 不会丢失。
        let def = ProcessDef {
            name: "p".into(), cmd: "cmd".into(),
            args: vec!["/c".into(), "ping -n 30 127.0.0.1".into()],
            cwd: None, env: HashMap::new(), autostart: false,
            restart: RestartConfig { policy: RestartPolicy::Never, ..Default::default() },
        };
        let state = AppState::new(cfg_with(def), PathBuf::from("perch.toml"));
        let sup = Supervisor::new(state.clone());

        for _ in 0..5 {
            sup.start("p").await.unwrap();
            // 几乎立刻 stop,尽量落在 spawn_process().await 完成前的窗口内。
            sup.stop("p").await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let rt = state.procs.read().await.get("p").cloned().unwrap();
            assert_eq!(rt.read().await.status, ProcStatus::Stopped);
            // stops 表中不应残留该进程的 token,否则下一轮 start() 会被误判为已在运行。
            assert!(!sup.stops.lock().await.contains_key("p"));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stop_immediately_after_start_stress_multi_thread() {
        // 多线程 runtime 下真实存在 spawn_process().await 与 stop() 并发执行的窗口
        // (单线程 runtime 下几乎不可能触发)。循环多次以增加命中该窗口的概率。
        let def = ProcessDef {
            name: "p".into(), cmd: "cmd".into(),
            args: vec!["/c".into(), "ping -n 30 127.0.0.1".into()],
            cwd: None, env: HashMap::new(), autostart: false,
            restart: RestartConfig { policy: RestartPolicy::Never, ..Default::default() },
        };
        let state = AppState::new(cfg_with(def), PathBuf::from("perch.toml"));
        let sup = Supervisor::new(state.clone());

        for i in 0..20 {
            sup.start("p").await.unwrap();
            sup.stop("p").await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;

            let rt = state.procs.read().await.get("p").cloned().unwrap();
            let status = rt.read().await.status;
            assert_eq!(status, ProcStatus::Stopped, "iteration {i}: status={status:?}");
        }
    }
}
