use crate::config;
use crate::core::metrics::spawn_metrics_collector;
use crate::core::scheduler::Scheduler;
use crate::core::state::AppState;
use crate::core::supervisor::Supervisor;
use crate::web::{self, WebContext};

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
            println!("子命令 '{other}' 将在阶段二实现");
        }
        None => {
            println!("交互菜单将在阶段二实现;当前可用: perch run");
        }
    }
    Ok(())
}

pub async fn run_foreground_until<F: std::future::Future<Output = ()>>(
    shutdown: F,
) -> anyhow::Result<()> {
    let path = config::default_path();
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
