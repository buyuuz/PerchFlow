use crate::core::state::SharedState;
use std::time::Duration;
use sysinfo::{Pid, System};

pub fn spawn_metrics_collector(state: SharedState) {
    tokio::spawn(async move {
        let mut sys = System::new();
        loop {
            sys.refresh_processes();
            let entries: Vec<(String, Option<u32>)> = {
                let procs = state.procs.read().await;
                let mut v = Vec::new();
                for (name, rt) in procs.iter() {
                    v.push((name.clone(), rt.read().await.pid));
                }
                v
            };
            for (name, pid) in entries {
                let (cpu, mem) = match pid {
                    Some(p) => sys
                        .process(Pid::from_u32(p))
                        .map(|proc_| (proc_.cpu_usage(), proc_.memory()))
                        .unwrap_or((0.0, 0)),
                    None => (0.0, 0),
                };
                if let Some(rt) = state.procs.read().await.get(&name).cloned() {
                    let mut r = rt.write().await;
                    r.cpu_percent = cpu;
                    r.mem_bytes = mem;
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}
