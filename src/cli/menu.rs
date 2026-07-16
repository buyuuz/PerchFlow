//! 交互菜单:无参数启动时显示,特权操作按需触发 UAC。

use crate::cli::elevate;
use std::io::{self, Write};

pub fn show() -> anyhow::Result<()> {
    loop {
        println!("\n=== Perch ===");
        println!("1) 安装服务 (Install)  2) 卸载 (Uninstall)  3) 启动 (Start)  4) 停止 (Stop)  5) 打开 Web UI (Open Web UI)  0) 退出 (Exit)");
        print!("选择 (Select): ");
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
            _ => println!("无效选择 (Invalid choice)"),
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
        println!("完成: {action} (Done)");
    } else {
        println!("需要管理员权限,正在请求提权... (Admin privileges required, elevating...)");
        elevate::relaunch_as_admin(action)?;
    }
    Ok(())
}

fn open_ui() -> std::io::Result<()> {
    // 端口被占用说明 server 已在跑;否则 fork 一个后台 `perch run` 子进程,
    // 等它起来再开浏览器。这样双击 exe → 选 5 → 页面可直接打开。
    let in_use = std::net::TcpListener::bind("127.0.0.1:9000").is_err();
    if !in_use {
        println!("启动服务中... (Starting server...)");
        let exe = std::env::current_exe()?;
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW (0x08000000): 不弹第二个控制台窗口
            std::process::Command::new(exe)
                .arg("run")
                .creation_flags(0x08000000)
                .spawn()?;
        }
        #[cfg(not(windows))]
        {
            std::process::Command::new(exe).arg("run").spawn()?;
        }
        // 给 server 2 秒起来
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    std::process::Command::new("cmd")
        .args(["/c", "start", "", "http://127.0.0.1:9000"])
        .spawn()
        .map(|_| ())
}
