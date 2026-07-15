use crate::core::state::{SharedState, ProcRuntime};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, System};
use tokio::sync::RwLock;

pub fn spawn_metrics_collector(state: SharedState) {
    tokio::spawn(async move {
        let mut sys = System::new();
        loop {
            sys.refresh_processes();
            let entries: Vec<(Arc<RwLock<ProcRuntime>>, Option<u32>)> = {
                let procs = state.procs.read().await;
                let mut v = Vec::new();
                for rt in procs.values() {
                    let pid = rt.read().await.pid;
                    v.push((rt.clone(), pid));
                }
                v
            };
            for (rt, pid) in entries {
                let (cpu, mem) = match pid {
                    Some(p) => sys
                        .process(Pid::from_u32(p))
                        .map(|proc_| (proc_.cpu_usage(), proc_.memory()))
                        .unwrap_or((0.0, 0)),
                    None => (0.0, 0),
                };
                let mut r = rt.write().await;
                r.cpu_percent = cpu;
                r.mem_bytes = mem;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}
