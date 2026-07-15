//! UAC 提权:检测当前是否具备管理员权限,以及以管理员身份重启自身。

#[cfg(windows)]
pub fn is_admin() -> bool {
    // 简易判断:`net session` 需要管理员权限才能成功执行。
    use std::process::Command;
    Command::new("net")
        .args(["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
pub fn relaunch_as_admin(arg: &str) -> crate::error::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let exe = std::env::current_exe()
        .map_err(|e| crate::error::PerchError::Service(format!("exe: {e}")))?;
    // 用 PowerShell Start-Process -Verb RunAs 触发 UAC
    Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Start-Process -FilePath '{}' -ArgumentList '{}' -Verb RunAs",
                exe.display(),
                arg
            ),
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| crate::error::PerchError::Service(format!("relaunch: {e}")))?;
    Ok(())
}
