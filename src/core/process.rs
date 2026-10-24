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
    // 脚本文件无法被 CreateProcess 直接执行,需要包装:
    // - .bat/.cmd: Rust std 已自动经 cmd.exe /c 执行,无需处理
    // - .ps1: 必须显式经 powershell.exe -File 调用,否则 spawn 报
    //   "%1 不是有效的 Win32 应用程序"(ERROR_BAD_EXE_FORMAT / os error 193)。
    //   包装后用户配置的 args 仍原样传给脚本。
    let (program, prefix_args): (String, Vec<String>) = match std::path::Path::new(&def.cmd)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("ps1") => (
            "powershell.exe".into(),
            vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                def.cmd.clone(),
            ],
        ),
        _ => (def.cmd.clone(), vec![]),
    };

    let mut cmd = Command::new(&program);
    cmd.args(&prefix_args).args(&def.args);
    if let Some(cwd) = &def.cwd {
        cmd.current_dir(cwd);
    } else if let Some(parent) = std::path::Path::new(&def.cmd).parent() {
        // cmd 是带路径的可执行文件(如 D:\AI\cmd\kiro-rs\start.bat)且用户未显式
        // 配 cwd 时,默认以其所在目录为工作目录,对齐"双击 bat"的直觉——否则子进程
        // 继承服务的工作目录(System32),bat/ps1 内的相对路径 .\xxx.exe 会找不到。
        // 这里始终按用户原始 cmd 推导(ps1 被包装成 powershell.exe 也不受影响)。
        // parent 为空表示 cmd 是走 PATH 的裸命令(如 "python"),保持继承不干预。
        if !parent.as_os_str().is_empty() {
            cmd.current_dir(parent);
        }
    }
    // 默认注入的 Python 相关环境变量,先设,便于用户在 [process.env] 中覆盖:
    // - PYTHONUNBUFFERED=1:stdout/stderr 被 pipe 时 Python 默认 block-buffered
    //   (4-8KB),日志会积在缓冲区不刷新,Perch 读不到;设此变量令其立即 flush。
    // - PYTHONUTF8=1 / PYTHONIOENCODING=utf-8:管道下 Python 默认按系统 ANSI
    //   (中文 Windows = GBK)输出,写中文/emoji 会触发 UnicodeEncodeError
    //   (如 loguru 报错甚至崩溃)。强制 UTF-8 输出,日志不再乱码。
    // 三者对非 Python 进程无副作用(它们不读这些变量)。
    cmd.env("PYTHONUNBUFFERED", "1");
    cmd.env("PYTHONUTF8", "1");
    cmd.env("PYTHONIOENCODING", "utf-8");
    // 用户显式配置的 env 放在最后,可覆盖上述默认值。
    for (k, v) in &def.env {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

/// 子进程 + (Windows 上)其所属的 Job Object。job 为 None 表示创建/分配失败,
/// 退回到 kill_tree 里 taskkill 的 fallback 路径。
pub struct SpawnedChild {
    pub child: Child,
    #[cfg(windows)]
    pub job: Option<crate::core::jobobj::JobObject>,
}

pub async fn spawn_process(
    def: &ProcessDef,
    runtime: Arc<RwLock<ProcRuntime>>,
) -> Result<SpawnedChild> {
    if let Some(ref cwd) = def.cwd {
        // Protect starts from stale/moved TOML paths, even when the UI was
        // not involved (e.g. a project directory removed after saving).
        if !std::path::Path::new(cwd).is_dir() {
            return Err(PerchError::Process(format!(
                "working directory is missing or is not a directory: {cwd}"
            )));
        }
    }
    let mut cmd = build_command(def);
    let mut child = cmd.spawn().map_err(|e| PerchError::Process(format!("spawn {}: {e}", def.name)))?;

    // spawn 后立即创建 job 并把子进程纳入其中。此刻子进程刚起步(uvicorn/python
    // 完成 import 并 fork 出孙进程需要时间),纳入 job 早于孙进程出生,
    // 孙进程会自动继承同一 job——这个窗口内产生的孤儿风险可忽略。
    #[cfg(windows)]
    let job = {
        match (crate::core::jobobj::JobObject::create(), child.raw_handle()) {
            (Ok(j), Some(h)) => match j.assign(h) {
                Ok(()) => Some(j),
                Err(e) => { tracing::warn!("assign job object failed for {}: {e}", def.name); None }
            },
            (Err(e), _) => { tracing::warn!("create job object failed for {}: {e}", def.name); None }
            (_, None) => None,
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let pid = child.id();

    {
        let mut rt = runtime.write().await;
        rt.status = ProcStatus::Running;
        rt.pid = pid;
        rt.started_at = Some(Instant::now());
        rt.cpu_percent = 0.0;
        rt.mem_bytes = 0;
    }

    if let Some(out) = stdout {
        spawn_reader(out, runtime.clone());
    }
    if let Some(err) = stderr {
        spawn_reader(err, runtime.clone());
    }
    #[cfg(windows)]
    return Ok(SpawnedChild { child, job });
    #[cfg(not(windows))]
    return Ok(SpawnedChild { child });
}

fn spawn_reader<R>(reader: R, runtime: Arc<RwLock<ProcRuntime>>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            // 过滤 ANSI 颜色/控制序列(如 \x1b[32m),否则前端显示为 [32m[1m 这样的乱码
            let cleaned = strip_ansi(&line);
            runtime.write().await.logs.push(cleaned);
        }
    });
}

/// 去除 ANSI 转义序列。支持 CSI 序列 `\x1b[...字母` 与 OSC 序列 `\x1b]...\x07`。
/// 其他单字符 ESC 序列(如 \x1b c)的 ESC 字符本身被丢弃,后续字符保留。
fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1; // 丢弃 ESC
            if i < bytes.len() && bytes[i] == b'[' {
                // CSI: \x1b[ ... 终止于 0x40-0x7E
                i += 1;
                while i < bytes.len() {
                    let c = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&c) {
                        break;
                    }
                }
            } else if i < bytes.len() && bytes[i] == b']' {
                // OSC: \x1b] ... 终止于 BEL(0x07) 或 ST(\x1b\\)
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            // 其他 ESC 序列:仅丢弃 ESC 字符,保留后续字符
        } else {
            // 安全地按 UTF-8 边界拷贝。ANSI 序列本身都是 ASCII,不会切断多字节字符。
            out.push(bytes[i]);
            i += 1;
        }
    }
    // 整行按 UTF-8 重建(原行就是合法 UTF-8,这里只是少了 ESC 序列字节)
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned())
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
            alias_zh: None,
            name: "echo".into(),
            cmd: "cmd".into(),
            args: vec!["/c".into(), "echo hello".into()],
            cwd: None,
            env: HashMap::new(),
            web_url: None,
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
        let mut spawned = spawn_process(&echo_def(), rt.clone()).await.unwrap();
        let _ = spawned.child.wait().await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let logs = rt.read().await.logs.recent(10);
        assert!(logs.iter().any(|l| l.contains("hello")), "logs={logs:?}");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn spawn_ps1_script_wrapped_with_powershell() {
        let dir = std::env::temp_dir().join(format!("perch_ps1_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("test_exit.ps1");
        std::fs::write(&script, "exit 42").unwrap();
        let def = ProcessDef {
            alias_zh: None,
            name: "ps1".into(),
            cmd: script.to_string_lossy().into_owned(),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            web_url: None,
            autostart: false,
            restart: RestartConfig::default(),
        };
        let rt = Arc::new(RwLock::new(ProcRuntime {
            status: ProcStatus::Stopped, pid: None, started_at: None,
            restart_count: 0, cpu_percent: 0.0, mem_bytes: 0,
            logs: crate::core::ringbuf::RingBuffer::new(100),
        }));
        let mut spawned = spawn_process(&def, rt.clone()).await.unwrap();
        let status = spawned.child.wait().await.unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(status.code(), Some(42));
    }

    #[test]
    fn strip_ansi_removes_color_codes() {
        let input = "\x1b[32m\x1b[1mVITE\x1b[22m\x1b[39m v6.3.5";
        assert_eq!(strip_ansi(input), "VITE v6.3.5");
    }

    #[test]
    fn strip_ansi_preserves_plain_text() {
        assert_eq!(strip_ansi("hello world"), "hello world");
        assert_eq!(strip_ansi("中文不乱码"), "中文不乱码");
    }

    #[test]
    fn strip_ansi_handles_osc_terminator() {
        let input = "\x1b]0;title\x07visible text";
        assert_eq!(strip_ansi(input), "visible text");
    }
}
