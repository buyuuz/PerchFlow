use crate::config::model::{ProcessDef, RestartPolicy};
use crate::core::backoff::Backoff;
use crate::core::process::{spawn_process, SpawnedChild};
use crate::core::state::{ProcStatus, SharedState};
use crate::error::{PerchError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// 一个正在被监督的进程实例。`cancel` 用于请求停止;`done` 在监督任务彻底
/// 结束(已写入终态并从注册表移除)后触发,使 stop() 能等待旧实例完全退场。
struct Instance {
    cancel: CancellationToken,
    done: CancellationToken,
}

pub struct Supervisor {
    state: SharedState,
    instances: Mutex<HashMap<String, Instance>>,
}

impl Supervisor {
    pub fn new(state: SharedState) -> Arc<Self> {
        Arc::new(Self { state, instances: Mutex::new(HashMap::new()) })
    }

    async fn find_def(&self, name: &str) -> Result<ProcessDef> {
        let cfg = self.state.config.read().await;
        cfg.process.iter().find(|p| p.name == name).cloned()
            .ok_or_else(|| PerchError::Process(format!("unknown process: {name}")))
    }

    /// 启动进程。注册表中存在同名实例即视为已在运行,直接返回。由于监督任务
    /// 是注册表唯一的移除者,且 stop() 会等待旧任务彻底退场后才返回,任一时刻
    /// 每个 name 至多只有一个存活实例——从而杜绝双实例并发写同一 runtime。
    pub async fn start(self: &Arc<Self>, name: &str) -> Result<()> {
        let def = self.find_def(name).await?;
        let runtime = {
            let procs = self.state.procs.read().await;
            procs.get(name).cloned()
                .ok_or_else(|| PerchError::Process(format!("no runtime: {name}")))?
        };
        let inst = Instance { cancel: CancellationToken::new(), done: CancellationToken::new() };
        let (cancel, done) = (inst.cancel.clone(), inst.done.clone());
        {
            let mut instances = self.instances.lock().await;
            if instances.contains_key(name) {
                return Ok(()); // 已在运行
            }
            instances.insert(name.to_string(), inst);
        }
        let this = self.clone();
        let name_owned = name.to_string();
        tokio::spawn(async move {
            this.supervise_loop(def, runtime, cancel, name_owned).await;
            done.cancel(); // 终态已写入、注册表已移除,通知等待方旧实例已退场
        });
        Ok(())
    }

    /// 请求停止并等待监督任务彻底结束。任务是终态的唯一写入者,故 stop() 返回
    /// 时 runtime 状态已确定,且注册表中该 name 已空,后续 start() 不会与旧任务重叠。
    pub async fn stop(&self, name: &str) -> Result<()> {
        let done = {
            let instances = self.instances.lock().await;
            match instances.get(name) {
                Some(inst) => { inst.cancel.cancel(); Some(inst.done.clone()) }
                None => None,
            }
        };
        if let Some(done) = done {
            done.cancelled().await;
        }
        Ok(())
    }

    pub async fn restart(self: &Arc<Self>, name: &str) -> Result<()> {
        self.stop(name).await?;
        self.start(name).await
    }

    /// Whether a supervise loop is currently registered for `name`.
    pub async fn is_managed(&self, name: &str) -> bool {
        self.instances.lock().await.contains_key(name)
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
        let names: Vec<String> = { self.instances.lock().await.keys().cloned().collect() };
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
        cancel: CancellationToken,
        name: String,
    ) {
        let mut backoff = Backoff::new(def.restart.backoff_secs, def.restart.backoff_max_secs);
        // 用带值的 break 把所有退出路径汇聚到循环末尾的单一权威终态写入点:
        // 取消路径一律为 Stopped,自然退出为真实状态。无论走哪条 break,终态都必被写入,
        // 且这次写入在时序上晚于本次迭代内 spawn_process / classify_exit 的所有中间写入。
        let terminal = loop {
            if cancel.is_cancelled() {
                break ProcStatus::Stopped;
            }
            match run_child(&def, &runtime, &cancel).await {
                ChildOutcome::SpawnError => break ProcStatus::Crashed,
                ChildOutcome::Cancelled => break ProcStatus::Stopped,
                ChildOutcome::Exited(status) => match classify_exit(&def, &runtime, status).await {
                    ExitAction::Done(s) => break s,
                    ExitAction::Restart => {
                        let delay = backoff.next_delay();
                        tokio::select! {
                            _ = tokio::time::sleep(delay) => {}
                            _ = cancel.cancelled() => break ProcStatus::Stopped,
                        }
                    }
                },
            }
        };
        {
            let mut r = runtime.write().await;
            r.status = terminal;
            r.pid = None;
        }
        // 监督任务是注册表唯一的移除者,且任一时刻每个 name 至多一个实例,
        // 故按 name 移除是安全的,不会误删其它实例。
        self.instances.lock().await.remove(&name);
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
    let mut spawned = match spawn_process(def, runtime.clone()).await {
        Ok(c) => c,
        Err(_) => return ChildOutcome::SpawnError,
    };
    if token.is_cancelled() {
        let _ = kill_tree(&mut spawned).await;
        return ChildOutcome::Cancelled;
    }
    tokio::select! {
        status = spawned.child.wait() => ChildOutcome::Exited(status),
        _ = token.cancelled() => {
            let _ = kill_tree(&mut spawned).await;
            ChildOutcome::Cancelled
        }
    }
}

/// 终止整个进程树。Job Object 存在时优先用 `TerminateJobObject` 一次性杀掉
/// job 内所有进程——包括 uvicorn --reload / multiprocessing 派生出的已 detach
/// 的孙进程(这些进程不再挂在父进程树上,`taskkill /T` 无法触达)。
/// 随后仍执行 `taskkill /T /F /PID` + `child.kill()` 作为兜底(job 缺失或
/// 分配失败时的原有路径,对已终止的进程无害)。
#[cfg(windows)]
async fn kill_tree(spawned: &mut SpawnedChild) -> std::io::Result<()> {
    if let Some(job) = &spawned.job {
        job.terminate();
    }
    let child = &mut spawned.child;
    if let Some(pid) = child.id() {
        let out = tokio::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
            .await?;
        if !out.status.success() {
            // taskkill 失败(例如 PID 已退出),退回到直接 kill
            let _ = child.kill().await;
        }
    }
    // 确保资源回收
    let _ = child.wait().await;
    Ok(())
}

#[cfg(not(windows))]
async fn kill_tree(spawned: &mut SpawnedChild) -> std::io::Result<()> {
    spawned.child.kill().await
}

/// 一次子进程退出后的决策:要么终结(带最终状态),要么退避后重启。
enum ExitAction {
    Done(ProcStatus),
    Restart,
}

/// 判定一次子进程退出后该终结还是重启。终结时返回最终状态交由循环末尾统一写入;
/// 重启时在此更新中间态(Restarting + 计数),超过重试上限则终结为 Crashed。
async fn classify_exit(
    def: &ProcessDef,
    runtime: &std::sync::Arc<tokio::sync::RwLock<crate::core::state::ProcRuntime>>,
    status: std::io::Result<std::process::ExitStatus>,
) -> ExitAction {
    let success = status.map(|s| s.success()).unwrap_or(false);
    if !should_restart(def.restart.policy, success) {
        return ExitAction::Done(if success { ProcStatus::Stopped } else { ProcStatus::Crashed });
    }
    let mut r = runtime.write().await;
    r.status = ProcStatus::Restarting;
    r.restart_count += 1;
    r.pid = None;
    if def.restart.max_retries != 0 && r.restart_count > def.restart.max_retries {
        return ExitAction::Done(ProcStatus::Crashed);
    }
    ExitAction::Restart
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
            server: ServerConfig { bind: "127.0.0.1:0".into(), token: "t".into(), lang: "en".into() },
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
            assert!(!sup.instances.lock().await.contains_key("p"));
        }
    }

    #[tokio::test]
    async fn stop_during_restart_backoff_marks_stopped() {
        // RestartPolicy::Always 且进程快速退出 → 进入重启退避循环。
        // 此时 stop() 必须让最终状态落到 Stopped,而非卡在 classify_exit 写入的 Restarting。
        // 用较长退避确保 stop() 落在 backoff 的 sleep 窗口内。
        let def = ProcessDef {
            name: "p".into(), cmd: "cmd".into(),
            args: vec!["/c".into(), "exit 1".into()],
            cwd: None, env: HashMap::new(), autostart: false,
            restart: RestartConfig {
                policy: RestartPolicy::Always,
                backoff_secs: 5, backoff_max_secs: 5, max_retries: 0,
            },
        };
        let state = AppState::new(cfg_with(def), PathBuf::from("perch.toml"));
        let sup = Supervisor::new(state.clone());
        sup.start("p").await.unwrap();
        // 让首个子进程退出并进入退避 sleep(5s)。
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        sup.stop("p").await.unwrap();
        // 给监督循环从 backoff 的 select! 中被取消唤醒并写入终态的时间。
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let rt = state.procs.read().await.get("p").cloned().unwrap();
        assert_eq!(rt.read().await.status, ProcStatus::Stopped);
        assert!(!sup.instances.lock().await.contains_key("p"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn restart_never_leaves_stopped_or_orphan() {
        // restart() = stop()+start()。因为 stop() 会等待旧监督任务彻底退场
        // (done 信号)才返回,新实例绝不会与旧实例重叠:旧任务的终态写入不会
        // 覆盖新实例的 Running,旧任务也不会误删新实例的注册表条目。
        // 多次 restart 后进程必须仍处于 Running。
        let def = ProcessDef {
            name: "p".into(), cmd: "cmd".into(),
            args: vec!["/c".into(), "ping -n 30 127.0.0.1".into()],
            cwd: None, env: HashMap::new(), autostart: false,
            restart: RestartConfig { policy: RestartPolicy::Never, ..Default::default() },
        };
        let state = AppState::new(cfg_with(def), PathBuf::from("perch.toml"));
        let sup = Supervisor::new(state.clone());
        sup.start("p").await.unwrap();
        for i in 0..5 {
            sup.restart("p").await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            let rt = state.procs.read().await.get("p").cloned().unwrap();
            let status = rt.read().await.status;
            assert_eq!(status, ProcStatus::Running, "iteration {i}: status={status:?}");
            assert!(sup.instances.lock().await.contains_key("p"), "iteration {i}: missing instance");
        }
        sup.stop("p").await.unwrap();
        let rt = state.procs.read().await.get("p").cloned().unwrap();
        assert_eq!(rt.read().await.status, ProcStatus::Stopped);
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
