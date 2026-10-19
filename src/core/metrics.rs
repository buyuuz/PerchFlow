use crate::core::state::{ProcRuntime, SharedState};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, System};
use tokio::sync::RwLock;

// Sum the resources of a managed process and all of its descendants.
// The values are sampled from the same sysinfo snapshot, so a parent
// and child are measured at approximately the same instant.
// Note: summing working sets may double-count shared physical pages.
fn process_tree_metrics(
    root: Pid,
    children: &HashMap<Pid, Vec<Pid>>,
    resources: &HashMap<Pid, (f32, u64)>,
) -> (f32, u64) {
    // Do not attribute a surviving child to a PID that already exited.
    if !resources.contains_key(&root) {
        return (0.0, 0);
    }

    let mut cpu = 0.0_f32;
    let mut memory = 0_u64;
    let mut visited = HashSet::new();
    let mut stack = vec![root];

    while let Some(pid) = stack.pop() {
        if !visited.insert(pid) {
            continue;
        }
        if let Some(&(process_cpu, process_memory)) = resources.get(&pid) {
            cpu += process_cpu;
            memory = memory.saturating_add(process_memory);
            if let Some(descendants) = children.get(&pid) {
                stack.extend(descendants.iter().copied());
            }
        }
    }

    (cpu, memory)
}

pub fn spawn_metrics_collector(state: SharedState) {
    tokio::spawn(async move {
        let mut sys = System::new();
        loop {
            sys.refresh_processes();

            // Build a single process-tree index per refresh. This preserves
            // Perch's low idle overhead even when many projects are managed.
            let mut children: HashMap<Pid, Vec<Pid>> = HashMap::new();
            let mut resources: HashMap<Pid, (f32, u64)> =
                HashMap::with_capacity(sys.processes().len());
            for (&pid, process) in sys.processes() {
                resources.insert(pid, (process.cpu_usage(), process.memory()));
                if let Some(parent) = process.parent() {
                    children.entry(parent).or_default().push(pid);
                }
            }

            let entries: Vec<(Arc<RwLock<ProcRuntime>>, Option<u32>)> = {
                let procs = state.procs.read().await;
                let mut v = Vec::with_capacity(procs.len());
                for rt in procs.values() {
                    let pid = rt.read().await.pid;
                    v.push((rt.clone(), pid));
                }
                v
            };

            for (rt, pid) in entries {
                let (cpu, mem) = pid
                    .map(|p| process_tree_metrics(Pid::from_u32(p), &children, &resources))
                    .unwrap_or((0.0, 0));
                let mut r = rt.write().await;
                r.cpu_percent = cpu;
                r.mem_bytes = mem;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(id: u32) -> Pid {
        Pid::from_u32(id)
    }

    #[test]
    fn sums_parent_child_grandchild_and_sibling() {
        let children = HashMap::from([
            (pid(10), vec![pid(11), pid(13)]),
            (pid(11), vec![pid(12)]),
        ]);
        let resources = HashMap::from([
            (pid(10), (1.0, 100)),
            (pid(11), (2.0, 200)),
            (pid(12), (3.0, 300)),
            (pid(13), (4.0, 400)),
            (pid(20), (99.0, 9000)),
        ]);
        let (cpu, memory) = process_tree_metrics(pid(10), &children, &resources);
        assert_eq!(cpu, 10.0);
        assert_eq!(memory, 1000);
    }

    #[test]
    fn excludes_unrelated_processes() {
        let children = HashMap::from([(pid(1), vec![pid(2)])]);
        let resources = HashMap::from([
            (pid(1), (1.0, 64)),
            (pid(2), (2.0, 128)),
            (pid(3), (50.0, 4096)),
        ]);
        assert_eq!(process_tree_metrics(pid(1), &children, &resources), (3.0, 192));
    }

    #[test]
    fn missing_root_returns_zero_even_if_a_child_remains() {
        let children = HashMap::from([(pid(1), vec![pid(2)])]);
        let resources = HashMap::from([(pid(2), (1.0, 100))]);
        assert_eq!(process_tree_metrics(pid(1), &children, &resources), (0.0, 0));
    }

    #[test]
    fn guard_against_duplicate_or_cyclic_parent_links() {
        let children = HashMap::from([
            (pid(1), vec![pid(2), pid(2)]),
            (pid(2), vec![pid(1)]),
        ]);
        let resources = HashMap::from([(pid(1), (1.0, 10)), (pid(2), (2.0, 20))]);
        assert_eq!(process_tree_metrics(pid(1), &children, &resources), (3.0, 30));
    }

    #[test]
    fn saturates_on_memory_overflow() {
        let children = HashMap::from([(pid(1), vec![pid(2)])]);
        let resources = HashMap::from([
            (pid(1), (0.0, u64::MAX)),
            (pid(2), (0.0, 1)),
        ]);
        assert_eq!(process_tree_metrics(pid(1), &children, &resources), (0.0, u64::MAX));
    }
}
