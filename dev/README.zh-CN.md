# PerchFlow 本地前端开发预览

## 使用方式

本目录的开发预览 **不需要重新编译 Rust、安装 npm 或更改正式 perch.exe**。
开发时，HTML、CSS、JS 都直接从仓库 `frontend/` 文件夹读取；修改后刷新浏览器即可看到更新。

- **双击 `dev/Preview-Start.vbs`**：无 CMD 窗口启动预览并打开网页
- **双击 `dev/Preview-Stop.vbs`**：仅退出预览，不退出正式 PerchFlow
- **预览地址**：http://127.0.0.1:9011/processes
- **预览范围**：`/`（登录）、`/processes`、`/process-detail?name=MailHub%20(DEMO)`、`/tasks`、`/assets/...`
- **项目目录**：`D:\Downloads\PerchFlow-patch-work`
- **修改源代码**：`frontend/*.html`、`frontend/assets/*.css`、`frontend/assets/*.js`

开发预览要求 Windows 和 Python 3.10+（当前电脑已具备）。服务只监听 `127.0.0.1:9011`，不会创建 Windows 计划任务或服务。

## 默认的模拟模式（推荐改 UI 使用）

默认是 `mock` 模式。页面和按钮可以展示项目、任务、CPU/内存、进程详情、模拟日志，模拟的启动／停止、增加／编辑／删除只会更改 **预览服务器内存里的示例数据**。

- 不会真实运行任何命令
- 不会修改正式 `perch.toml`
- 不会连接或影响正式的 MailHub、PerchFlow
- 关闭并重新打开预览会重置示例数据
- 无需输入正式服务访问令牌，直接访问 `/processes` 即可

如果想预览登录页面，可访问 `/`。模拟模式的登录仅作外观测试，不验证真实令牌。页面左下角有 `DEV PREVIEW · MOCK DATA` 提示，该标记由预览服务注入，并不会被编译到正式 EXE 中。

## 可选：只读的实时数据

如需使用正式后端的实际进程数据检查 UI，可以 **另开** 一个本地预览实例：

```powershell
python dev/preview_server.py --port 9012 --mode live-readonly --backend http://127.0.0.1:9002
```

随后打开 http://127.0.0.1:9012/processes 。

此模式使用浏览器中的授权令牌进行同源 API 转发；只允许 GET 读操作，所有 POST / PUT / DELETE 均返回 403，避免影响正式项目。需要登录时，可在该预览端口的登录页手动输入令牌；预览服务不保存令牌。日志详情支持读取快照，**不会代理实时 SSE 连接**。不需要实时数据时，使用默认模拟模式更安全。

## 运行测试

在仓库根目录执行：

```powershell
python -m unittest discover -s dev -p test_*.py -v
```

无需安装额外 Python 包。测试不会修改正式服务或配置。

## GitHub 工作流

- 开发分支：`feature/frontend-dev-preview`，从已包含进程树资源统计修复的 `fix/process-tree-metrics` 分支派生
- 日常修改 `frontend/`，刷新预览，不编译 Rust
- 设计定稿后先提交 Git，再创建一个专门的 `ui-build-*` 标签并推送；`.github/workflows/build-ui-windows.yml` 才会运行
- Actions 将验证 Rust 和预览测试，随后打包带新 UI 的单文件 `perch.exe` 供下载。正式版本继续采用 `RustEmbed`，**不依赖 Python 预览服务**

示例（**UI 定稿前不要执行**）：

```powershell
git add frontend dev .github/workflows/build-ui-windows.yml
git commit -m "style: refresh PerchFlow UI"
git push origin feature/frontend-dev-preview
git tag ui-build-20261019
git push origin ui-build-20261019
```

## 注意

1. 源码中的文件与现有已安装 EXE 是两个版本；预览页面变化不会修改当前正在运行的 Perch。
2. 不要把实际 `perch.toml` 的令牌、邮箱或其他凭证提交到 GitHub。
3. 正式 EXE 更新前先备份，并正常停止托管项目，然后替换 EXE。
4. 只允许在受信任的本机进行预览；默认模拟模式没有真正的访问认证。
