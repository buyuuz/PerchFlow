<div align="center">
<sub>🇬🇧 <a href="./README.md">English</a> &nbsp;|&nbsp; 🇨🇳 <a href="./README.zh-CN.md">中文</a> &nbsp;|&nbsp; 🇪🇸 <a href="./README.es.md">Español</a> &nbsp;|&nbsp; 🇯🇵 <a href="./README.ja.md">日本語</a> &nbsp;|&nbsp; 🇧🇷 <a href="./README.pt.md">Português</a></sub>
</div>

<br/>

<div align="center">

<img src="logo.png" alt="Perch" width="120">

# Perch

**轻量级 Windows 进程与服务管理器，单文件、内置 Web UI。**

Rust 编写，无运行时依赖。一个可执行文件，双击即用。

</div>

---

## 截图展示

> 将截图文件放到下文各路径下（待提供文件清单见 [需要提供的截图文件](#需要提供的截图文件)）。

### 交互菜单（双击 `perch.exe`）

![menu](docs/screenshots/menu.png)

### 登录页

![login](docs/screenshots/login.png)

### 进程面板

![processes](docs/screenshots/processes.png)

### 进程详情 —— 实时日志流

![process-detail](docs/screenshots/process-detail.png)

### 定时任务

![tasks](docs/screenshots/tasks.png)

---

## 为什么做 Perch？

在 Windows 上管理后台进程和常驻服务，比想象中麻烦得多：

- **任务计划程序（Task Scheduler）** 默认会杀掉长时间运行的进程（会话超时、进程树终止、依赖登录会话）——它本就不是为守护进程设计的。
- **NSSM / WinSw / Shawl** 都是一个包装器管一个服务。没有面板、没有全局视图，要注册 N 次。
- **PM2** 依赖 Node.js 运行时，守护进程空载就占 40–80 MB，且在 Windows 上是二等公民（自启动不可靠、要 hack 注册表）。
- **Supervisor** 根本不支持 Windows。

Perch 填补了这个空白：**注册一次，管理全部。** 单个原生服务看管所有进程，提供 Web UI，安静地待在后台。

## 特性

- **单文件** —— 无运行时、无依赖。Web UI 直接内嵌进可执行文件。
- **原生 Windows 服务** —— 注册一次，开机自启，注销不停，运行于 Session 0。
- **极低占用** —— Rust 编译，单个守护进程，目标常驻内存 5–15 MB。
- **内置 Web UI** —— 浏览器里查看状态、启停/重启进程、查看日志。支持远程管理，不必守在机器前。
- **崩溃自动拉起** —— 进程挂掉自动重启，退避策略可配置。
- **定时任务** —— cron 风格的周期任务调度。
- **按日切分日志** —— 应用日志写到可执行文件同级的 `logs/perch.YYYY-MM-DD.log`，管道分隔格式。
- **交互菜单** —— 双击 exe 即可安装/卸载/启动/停止服务、打开 Web UI，特权操作按需触发 UAC 提权。

## 项目状态

> ⚠️ **早期开发中。** Perch 正在积极建设，尚未达到生产可用。接口与配置可能变动。

## 快速开始

双击 `perch.exe`，在菜单中选 `5) 打开 Web UI (Open Web UI)`——Perch 会自动生成默认 `perch.toml`（含 `demo` 进程和 token `change-me`）并启动服务。

或命令行启动：

```
# 前台运行（Ctrl-C 停止）
perch run

# 注册为 Windows 服务（需管理员）
perch install
perch start

# 打开 Web UI
http://127.0.0.1:9000
```

默认 token 是 `change-me`——正式使用前请改 `perch.toml` 里的 `server.token`。

## 安全

管理端点可以启停任意进程，**权限极高**。Perch 默认仅监听 `127.0.0.1` 并要求访问令牌。不要将管理端口暴露到不可信网络。

## 对比

| | Perch | NSSM / WinSw | PM2 | Supervisor |
|---|:---:|:---:|:---:|:---:|
| 单文件 | ✅ | ✅ | ❌（需 Node） | ❌（需 Python） |
| 单实例管理多进程 | ✅ | ❌（一个一包装） | ✅ | ✅ |
| 内置 Web UI | ✅ | ❌ | ✅ | ✅ |
| 原生 Windows 服务 | ✅ | ✅ | ⚠️ | ❌ |
| 定时任务 | ✅ | ❌ | ✅ | ⚠️ |
| 按日切分应用日志 | ✅ | ❌ | ⚠️ | ⚠️ |
| 低内存占用 | ✅ | ✅ | ❌ | ⚠️ |

## 从源码构建

```
cargo build --release
```

需要较新的 stable Rust 工具链。

## 需要提供的截图文件

将截图放到 `docs/screenshots/` 目录下（先创建该目录），使用以下精确文件名：

| 路径 | 截图内容 |
|---|---|
| `docs/screenshots/menu.png` | 双击 `perch.exe` 后的控制台窗口，显示中英对照菜单（安装/卸载/启动/停止/打开 Web UI/退出）。 |
| `docs/screenshots/login.png` | `http://127.0.0.1:9000/` 登录页，含 Perch logo 和 token 输入框。 |
| `docs/screenshots/processes.png` | `/processes` 进程面板，含 `demo` 进程卡片显示 Running 状态，顶栏含 logo。 |
| `docs/screenshots/process-detail.png` | `/process-detail?name=demo` 页面，含 PID/CPU/内存/运行时间面板和实时日志流。 |
| `docs/screenshots/tasks.png` | `/tasks` 定时任务页，至少一行任务，顶栏含 logo。 |

建议：浏览器视口 1280×800，PNG 格式，裁掉浏览器边框。文件放好后，上面的图片引用会自动渲染。

## 许可证

待定
