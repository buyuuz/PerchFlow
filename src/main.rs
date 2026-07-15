fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = perch::cli::run() {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}
