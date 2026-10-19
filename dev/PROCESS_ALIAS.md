# PerchFlow · 进程双语名称（开发预览）

更新时间：2026-10-19
分支：`feature/frontend-dev-preview`
范围：添加／编辑进程、进程列表、进程详情、Rust 配置序列化、API 和开发 Mock。

## 用户最终确认的展示方式

进程的英文名称始终是主要名称（较大、加粗），中文别名始终是下方的较小副标题。

示例：

**MailHub**（13.5px）
邮箱管理中心（11px）

无论界面语言选择“中文”还是 “English”，这两个名称的顺序都固定。

- `name`：原有英文主名称，仍是进程生命周期和 API 的唯一键。不自动翻译，也不修改现有的进程 ID、日志路径与操作 endpoint。
- `alias_zh`：可选中文显示别名。没有别名的旧项目，只显示英文主名称；不生成冗余空白占位。
- 添加及编辑弹窗均提供中文别名输入框，允许留空。限制长度为 80 字符。
- 进程列表搜索同时支持英文名和中文别名。
- 进程详情页使用原英文 h1 标题，中文别名以更小文字排在其下。
- HTML 字符均按文本安全转义，防止别名中的标签成为可执行 HTML。

## 持久化和 API

Rust `ProcessDef` 新增 `alias_zh: Option<String>` 并配置 `serde(default, skip_serializing_if="Option::is_none")`。
因此旧的 `perch.toml` 不含 `alias_zh` 也可正常读取；新增或编辑别名后，该字段会保存到配置。

`GET /api/processes`、`GET /api/processes/{name}` 的 `ProcView` 均增加 `alias_zh`。
`POST /api/processes`、`PUT /api/processes/{name}` 继续使用现有路由，新增字段为可选。

新增 `PUT /api/processes/{name}/alias`，接收 `{"alias_zh":"邮箱管理中心"}`。
仅调整中文别名时前端调用此专用路由：只修改与持久化配置，不调用 supervisor，也不会停止/重启运行中的进程；传 `null` 或空字符串可以清除别名。

如果同时修改了命令、参数或自动启动选项，则仍按原有完整 PUT 更新逻辑处理。
英文主名称在编辑模式下保持不可变（已有约束），改名需另建进程。

## 验证和安全边界

- `python -m unittest discover -s dev -p test_*.py -q`
- `python dev/verify_process_alias_browser.py`
- `python dev/verify_process_table_browser.py`
- `python dev/verify_process_detail_browser.py`
- `python dev/verify_tasks_table_browser.py`
- `python dev/verify_topbar_controls_browser.py`
- `python dev/verify_nav_stability.py`
- `node --check frontend/assets/processes.js`
- `node --check frontend/assets/process-detail.js`

浏览器操作和变更只针对 **127.0.0.1:9011 的 Mock 模式**。用户已经部署的 9002 正式 EXE 和既有便携 ZIP 在用户看过并确认新版样式之前不替换。

**注意：本次涉及 Rust 新字段与新 API；只更改前端文件并不足以把别名永久保存到正式版。正式使用前需要重新编译新 Rust EXE 并按旧版安全升级流程部署。**
