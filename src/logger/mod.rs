use std::path::PathBuf;
use chrono::Local;
use tracing::field::Visit;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{
        self,
        format::{Writer, FormatEvent, FormatFields},
        FmtContext,
    },
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_subscriber::registry::LookupSpan;

/// 获取可执行文件所在目录
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// 初始化日志系统
///
/// 同时输出到控制台和文件,格式:{时间}|{级别}|{文件}:{行号}|{消息}
/// 日志文件按日切分,存储在可执行文件同级 logs/ 目录
/// 文件名格式:{app_name}.YYYY-MM-DD.log
pub fn init_logger(app_name: &str, level: &str) -> anyhow::Result<()> {
    let log_dir = exe_dir().join("logs");
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| anyhow::anyhow!("创建日志目录失败 ({}): {}", log_dir.display(), e))?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(app_name)
        .filename_suffix("log")
        .build(&log_dir)
        .map_err(|e| anyhow::anyhow!("创建日志文件失败: {}", e))?;

    let file_layer = fmt::Layer::new()
        .with_ansi(false)
        .with_writer(file_appender)
        .event_format(PipeFormat);

    let console_layer = fmt::Layer::new()
        .with_ansi(false)
        .with_writer(std::io::stdout as fn() -> std::io::Stdout)
        .event_format(PipeFormat);

    // RUST_LOG 优先,否则用传入的 level
    let env_filter = if let Ok(filter) = EnvFilter::try_from_env("RUST_LOG") {
        filter
    } else {
        EnvFilter::new(level)
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("初始化日志失败: {}", e))?;

    Ok(())
}

/// 管道分隔符格式器
/// 输出格式:{时间}|{级别}|{文件}:{行号}|{消息}
struct PipeFormat;

impl<S, N> FormatEvent<S, N> for PipeFormat
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let now = Local::now();
        write!(writer, "{}", now.format("%Y-%m-%d %H:%M:%S"))?;
        write!(writer, "|")?;

        let level = event.metadata().level();
        write!(writer, "{level}")?;
        write!(writer, "|")?;

        if let Some(file) = event.metadata().file() {
            write!(writer, "{file}")?;
            if let Some(line) = event.metadata().line() {
                write!(writer, ":{line}")?;
            }
        }
        write!(writer, "|")?;

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        if let Some(msg) = visitor.message {
            write!(writer, "{msg}")?;
        }

        writeln!(writer)
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}
