# Perch MVP 设计文档

日期:2026-07-15
状态:已评审,待实现

## 1. 目标与范围

Perch 是一个轻量级 Windows 进程与服务管理器,单文件、内置 Web UI,Rust 编写、无运行时依赖。

本 MVP 覆盖:

- 注册为 Windows 服务(install/uninstall),开机自启,Session 0 运行。
- 从单个 TOML 配置文件读取进程定义,启动/监控/重启多进程。
- 崩溃自动拉起(靠退出检测,非探针),可配置退避策略。
- 内置 Web UI(原生 HTML/JS 内嵌进二进制):查看状态、启停/重启进程、查看日志。
- **Web UI 增删改进程定义(添加服务),API 写回 TOML。**
- **定时任务(cron):Web UI 增删改定时任务,到点触发一次性子进程。**
- **资源/状态监控展示:CPU、内存、运行时长、重启次数。**
- 无参数双击 exe 进入控制台交互菜单,特权操作自动 UAC 提权。

不在本 MVP 范围(留待后续里程碑):存活探针(卡死检测)、通知、日志落盘持久化。

## 2. 关键技术决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| MVP 范围 | 核心监督 + Web UI + 完整服务集成 | 一次做出接近愿景的自闭环系统 |
| 配置方式 | 单个 TOML 文件(exe 同目录 `perch.toml`),API 可读写 | Rust 生态事实标准,人可读可手改;API 增删改后原子写回 |
| 监控含义 | 资源/状态展示(CPU/内存/运行时长/重启次数) | 用 sysinfo 采集,存活探针留待后续 |
| 定时任务 | cron 表达式触发一次性子进程 | 到点 spawn,退出即结束,不监督不重启 |
| 日志存储 | 仅内存环形缓冲(每进程最近 N 行) | 最轻,重启丢日志为可接受取舍 |
| 前端 | 原生 HTML/JS,rust-embed 内嵌 | 无 npm/node 构建链,cargo build 一步到位 |
| 服务化 | MVP 即含完整 Windows 服务集成 | install/uninstall、Session 0、开机自启 |
| 并发架构 | 单个 Tokio 运行时(axum + tokio::process + 定时器) | Rust 事实标准,HTTP/进程监督/定时统一并发模型 |
| 双击行为 | 控制台交互菜单 + 自动 UAC 提权 | 保留双击即用,同时保留子命令给脚本 |

## 3. 总体架构

单个二进制,通过子命令区分角色:

- 无参数运行 → 控制台交互菜单(1 安装 / 2 卸载 / 3 启动 / 4 停止 / 5 打开 Web UI / 0 退出)。
- `perch install` / `uninstall` → 注册/注销 Windows 服务(SCM API)。
- `perch run` → 前台运行 supervisor,用于调试。
- `perch start` / `stop` → 启停已注册的服务。
- 由 SCM 拉起 → 服务模式,走 windows-service dispatcher。

**UAC 提权**:执行特权操作时检测当前是否管理员,非管理员则以 `runas` 自我提权重启并透传对应子命令参数。

**核心运行时**:单个 tokio 运行时承载三件事——
1. Supervisor:按配置启动子进程,监控退出,按退避重启,异步读 stdout/stderr 进内存 ring buffer。
2. HTTP server(axum):内嵌静态前端 + JSON API,绑定 `127.0.0.1:9000`,token 鉴权。
3. 共享状态中枢 `AppState`:supervisor 写、API 读,唯一共享可变状态。

**服务桥接**:service dispatcher 在独立线程接收 SCM 的 stop 信号,通过 channel 通知 tokio 主循环优雅关停(先停 Web,再终止所有子进程)。

## 4. 组件划分与数据流

- **cli/entry**:参数解析与分派,UAC 提权。
- **menu**:控制台交互菜单,仅负责 IO 与调用,不含业务逻辑。
- **service**:Windows 服务集成(SCM install/uninstall、dispatcher、控制信号),经 channel 与 core 通信。
- **config**:加载/解析/校验 TOML,定义 `Config`、`ProcessDef`、`RestartPolicy`、`TaskDef`;原子写回(temp file + rename),避免与手改冲突。
- **core/supervisor**:持有 `Arc<AppState>`(进程表:运行态、pid、重启次数、CPU/内存、ring buffer);启动子进程、读日志、监控退出、退避重启、响应 API 指令。
- **core/scheduler**:cron 调度器,按 `TaskDef` 的表达式到点 spawn 一次性子进程,输出进 ring buffer,退出即结束。
- **core/metrics**:用 `sysinfo` 按 pid 定期(约 2s)采集 CPU/内存,刷新进 AppState。
- **web**:axum server,内嵌前端、JSON API(含进程/任务 CRUD)、token 中间件、SSE 日志推送。

数据流:

- 启动进程:菜单/API → supervisor 指令 → spawn 子进程 → 派生日志读取 task → 更新 AppState。
- 看状态:浏览器 → GET /api/processes → 读 AppState(含 CPU/内存)→ JSON。
- 看日志:浏览器 → GET /api/processes/:name/logs → 读该进程 ring buffer → 最近 N 行。
- 增删改进程/任务:浏览器 → API → 校验 → 改内存 Config → 原子写回 TOML → 通知 supervisor/scheduler 重载。
- 定时任务触发:scheduler 定时器到点 → spawn 一次性子进程 → 输出进 ring buffer。
- 服务停止:SCM → dispatcher → channel → supervisor 优雅关停 → 进程退出。

`AppState` 用 `Arc` + 每进程独立锁(`RwLock`),避免全局大锁。

## 5. 目录结构(企业级分层)

```
perch/
├── Cargo.toml
├── perch.toml                  # 示例配置
├── build.rs                    # 内嵌 manifest(requireAdministrator)
├── src/
│   ├── main.rs                 # 入口,极薄,只调 cli
│   ├── lib.rs                  # 库根,导出各模块(便于集成测试)
│   ├── cli/
│   │   ├── mod.rs              # 参数解析、分派
│   │   ├── menu.rs             # 交互菜单
│   │   └── elevate.rs          # UAC 提权
│   ├── config/
│   │   ├── mod.rs
│   │   ├── model.rs            # Config / ProcessDef / RestartPolicy / TaskDef
│   │   └── writer.rs           # 原子写回 TOML
│   ├── core/
│   │   ├── mod.rs
│   │   ├── supervisor.rs       # 监督循环
│   │   ├── backoff.rs          # 重启退避计算(从 supervisor 拆出)
│   │   ├── process.rs          # 单进程句柄、日志读取
│   │   ├── scheduler.rs        # cron 定时任务调度
│   │   ├── metrics.rs          # sysinfo 资源采集
│   │   ├── state.rs            # AppState 共享状态
│   │   └── ringbuf.rs          # 内存环形缓冲
│   ├── service/
│   │   ├── mod.rs              # install/uninstall
│   │   └── runner.rs           # service dispatcher + SCM 信号
│   ├── web/
│   │   ├── mod.rs              # axum server 装配
│   │   ├── api.rs              # JSON API handlers
│   │   ├── auth.rs             # token 中间件
│   │   └── assets.rs           # rust-embed 静态资源
│   └── error.rs                # 统一错误类型
├── frontend/                   # Stitch 产出的静态页面(内嵌进二进制)
│   └── (index.html / js / css)
└── tests/
    ├── config_test.rs
    └── api_test.rs
```

## 6. 编码规范(硬约束)

- **单个文件不超过 400 行**,超出必须按职责拆分到新文件。
- **单个方法/函数不超过 50 行**,超出必须拆解为更小的函数。
- 最小代码解决问题,不加未要求的功能、抽象、配置项。
- 匹配现有风格,自己产生的孤儿代码要清理。

## 7. 配置文件示例(perch.toml)

```toml
[server]
bind = "127.0.0.1:9000"
token = "change-me"          # Web UI / API 访问令牌

[[process]]
name = "my-app"
cmd = "C:\\apps\\my-app.exe"
args = ["--port", "8080"]
cwd = "C:\\apps"
env = { RUST_LOG = "info" }
autostart = true

[process.restart]
policy = "on-failure"        # always | on-failure | never
backoff_secs = 2             # 初始退避
backoff_max_secs = 60        # 退避上限
max_retries = 0              # 0 = 无限

[[task]]
name = "nightly-backup"
cmd = "C:\\scripts\\backup.bat"
args = []
cwd = "C:\\scripts"
schedule = "0 0 3 * * *"     # cron 表达式(秒 分 时 日 月 周)
enabled = true
```

## 8. 错误处理

- 库内部用 `thiserror` 定义 `PerchError`(Config / Process / Service / Web 变体)。
- `main` / CLI 边界用 `anyhow` 兜底并友好打印。
- 子进程崩溃不是错误,是 supervisor 正常事件流,走重启逻辑而非 `Result::Err`。

## 9. 测试策略

- 单元:config 解析/校验、TOML 原子写回、ringbuf 行为、退避计算、cron 表达式解析。
- 集成(`tests/`):起真实 supervisor 管理 dummy 子进程(如 `cmd /c ping`),验证启停/重启/日志;用 axum test client 打 API 验证契约(含进程/任务 CRUD 与写回)。
- 服务安装类需管理员权限,MVP 手动验证,不进 CI。

## 9.1 前端页面(共 5 个,供 Stitch 设计)

1. **Token 登录入口** — 首次访问输入 token,校验后存 localStorage,后续请求带 `Authorization` 头。
2. **进程列表(主仪表盘)** — 每个进程展示状态、CPU/内存、运行时长、重启次数、命令;行内启停/重启/日志操作;"添加进程"按钮。
3. **进程详情 / 日志** — 元信息面板 + 终端风格实时日志(SSE live 切换、行数选择)。
4. **定时任务列表** — 每个任务展示 cron 表达式、命令、启用状态、上次运行时间/退出码;立即运行/编辑/删除;"添加任务"按钮。
5. **添加/编辑表单** — 进程与任务共用的弹窗表单,字段对应各自定义。

## 9.2 主要依赖(crate)

`tokio`、`axum`、`serde` / `toml`、`rust-embed`、`windows-service`、`sysinfo`、`cron`(或 `tokio-cron-scheduler`)、`thiserror`、`anyhow`。

## 10. API 契约(供前端对接)

所有 `/api/*` 需请求头 `Authorization: Bearer <token>`。

进程:

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/processes` | 进程列表 + 状态 + 资源指标 |
| GET | `/api/processes/:name` | 单进程详情 |
| POST | `/api/processes` | 新增进程定义(写回 TOML) |
| PUT | `/api/processes/:name` | 编辑进程定义 |
| DELETE | `/api/processes/:name` | 删除进程定义 |
| POST | `/api/processes/:name/start` | 启动 |
| POST | `/api/processes/:name/stop` | 停止 |
| POST | `/api/processes/:name/restart` | 重启 |
| GET | `/api/processes/:name/logs?lines=N` | 最近 N 行日志 |
| GET | `/api/logs/stream/:name` | SSE 实时日志流 |

定时任务:

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/tasks` | 任务列表 |
| POST | `/api/tasks` | 新增任务 |
| PUT | `/api/tasks/:name` | 编辑任务 |
| DELETE | `/api/tasks/:name` | 删除任务 |
| POST | `/api/tasks/:name/run` | 立即触发一次 |

进程状态 JSON:

```json
{
  "name": "my-app",
  "status": "running",
  "pid": 12345,
  "uptime_secs": 3600,
  "restart_count": 0,
  "cpu_percent": 1.2,
  "mem_bytes": 15728640,
  "cmd": "C:\\apps\\my-app.exe --port 8080"
}
```

`status` 取值:`running` | `stopped` | `crashed` | `restarting`。

任务 JSON:

```json
{
  "name": "nightly-backup",
  "schedule": "0 0 3 * * *",
  "cmd": "C:\\scripts\\backup.bat",
  "enabled": true,
  "last_run": "2026-07-15T03:00:00Z",
  "last_exit_code": 0
}
```

进程/任务 CRUD 请求体为对应定义的 JSON(字段同 `perch.toml` 的 `[[process]]` / `[[task]]`)。

列表响应:`{ "processes": [ ... ] }` / `{ "tasks": [ ... ] }`。

日志响应:`{ "name": "my-app", "lines": ["...", "..."] }`。

操作类响应(start/stop/restart/run/CRUD):`{ "ok": true, "status": "running" }`,失败 `{ "ok": false, "error": "..." }`。

## 11. 安全

- 管理端点权限极高(可启停任意进程),默认仅监听 `127.0.0.1`。
- 所有 API 需 token 鉴权,token 存于 `perch.toml`。
- 不将管理端口暴露到不可信网络。
