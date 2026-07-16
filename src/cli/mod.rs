use crate::config;
use crate::config::writer;
use crate::core::metrics::spawn_metrics_collector;
use crate::core::scheduler::Scheduler;
use crate::core::state::AppState;
use crate::core::supervisor::Supervisor;
use crate::web::{self, WebContext};

pub mod elevate;
pub mod menu;

pub fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("run") => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(run_foreground_until(async {
                let _ = tokio::signal::ctrl_c().await;
                println!("收到 Ctrl-C,正在关停...");
            }))?;
        }
        #[cfg(windows)]
        Some("install") => crate::service::install()?,
        #[cfg(windows)]
        Some("uninstall") => crate::service::uninstall()?,
        #[cfg(windows)]
        Some("start") => crate::service::start()?,
        #[cfg(windows)]
        Some("stop") => crate::service::stop()?,
        #[cfg(windows)]
        Some("service") => {
            crate::service::runner::run_as_service().map_err(|e| anyhow::anyhow!(e))?;
        }
        Some(other) => {
            println!("未知子命令 '{other}'");
        }
        None => {
            menu::show()?;
        }
    }
    Ok(())
}

pub async fn run_foreground_until<F: std::future::Future<Output = ()>>(
    shutdown: F,
) -> anyhow::Result<()> {
    let path = config::default_path();
    // 首次运行:exe 同目录无 perch.toml 时,自动生成含 demo 进程的默认配置,
    // 这样双击 exe → 选 5 → fork `perch run` 时不会因缺配置崩溃退出。
    if !path.exists() {
        let cfg = config::model::default_demo_config();
        writer::save(&path, &cfg)?;
        println!("已生成默认配置: {}", path.display());
    }
    let cfg = config::load(&path)?;
    let bind = cfg.server.bind.clone();
    let state = AppState::new(cfg, path);

    let supervisor = Supervisor::new(state.clone());
    let scheduler = Scheduler::new(state.clone());
    spawn_metrics_collector(state.clone());
    supervisor.start_autostart().await;
    scheduler.start().await;

    let ctx = WebContext { state, supervisor: supervisor.clone(), scheduler: scheduler.clone() };

    println!("Perch 运行中: http://{bind}");
    tokio::select! {
        r = web::serve(ctx, &bind) => { r?; }
        _ = shutdown => {}
    }
    supervisor.shutdown().await;
    scheduler.shutdown().await;
    Ok(())
}
