//! 交互菜单:无参数启动时显示,特权操作按需触发 UAC。

use crate::cli::elevate;
use std::io::{self, Write};

pub fn show() -> anyhow::Result<()> {
    loop {
        println!("\n=== Perch ===");
        println!("1) 安装服务  2) 卸载  3) 启动  4) 停止  5) 打开 Web UI  0) 退出");
        print!("选择: ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break; // stdin 关闭(EOF / 非交互环境),退出避免空读死循环
        }
        match line.trim() {
            "1" => privileged("install")?,
            "2" => privileged("uninstall")?,
            "3" => privileged("start")?,
            "4" => privileged("stop")?,
            "5" => {
                let _ = open_ui();
            }
            "0" => break,
            _ => println!("无效选择"),
        }
    }
    Ok(())
}

fn privileged(action: &str) -> anyhow::Result<()> {
    if elevate::is_admin() {
        match action {
            "install" => crate::service::install()?,
            "uninstall" => crate::service::uninstall()?,
            "start" => crate::service::start()?,
            "stop" => crate::service::stop()?,
            _ => {}
        }
        println!("完成: {action}");
    } else {
        println!("需要管理员权限,正在请求提权...");
        elevate::relaunch_as_admin(action)?;
    }
    Ok(())
}

fn open_ui() -> std::io::Result<()> {
    std::process::Command::new("cmd")
        .args(["/c", "start", "http://127.0.0.1:9000"])
        .spawn()
        .map(|_| ())
}
