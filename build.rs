fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("logo.ico");
        res.set_manifest_file("perch.exe.manifest");
        // exe 属性面板里显示的信息(中英对照)
        res.set("FileDescription", "Perch - 轻量级 Windows 进程与服务管理器 (Lightweight Windows Process & Service Manager)");
        res.set("ProductName", "Perch");
        res.set("LegalCopyright", "© 2026 Perch Project");
        if let Err(e) = res.compile() {
            eprintln!("winres compile failed: {}", e);
            std::process::exit(1);
        }
    }
}
