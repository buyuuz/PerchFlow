fn main() {
    if let Err(e) = perch::logger::init_logger("perch", "info") {
        eprintln!("日志初始化失败: {e}");
        // 退回到默认 fmt,保证控制台至少有输出
        let _ = tracing_subscriber::fmt::try_init();
    }
    if let Err(e) = perch::cli::run() {
        tracing::error!("主流程错误: {e}");
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}
