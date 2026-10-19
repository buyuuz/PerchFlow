# PerchFlow · 进程工作目录（Working Directory）

状态：开发预览版，2026-10-19。源码分支：`feature/frontend-dev-preview`。

## 目的

为「添加进程」和「编辑进程」窗口开放已有 `ProcessDef.cwd`，
避免配置、相对路径文件和数据目录从 PerchFlow 自身目录解析。

示例：wb2api

- 命令：`D:\Projects\wb2api-panel-v1.13.0\wb2api.exe`
- 参数：`-config config.json`
- 工作目录：`D:\Projects\wb2api-panel-v1.13.0`

上面是用户提出的路径示例，使用前请确认目标电脑实际存在该目录及配置文件。

## 界面与交互约定

- 在原有命令、参数之后新增 **工作目录（可选）** 输入框，最大长度 1024。
- 编辑现有项目时回填 API 的 `cwd`。留空映射为 `null`，不填写时兼容旧项目。
- 当命令是绝对路径的 `.exe` 时，显示「EXE 所在文件夹」建议及「使用建议」按钮，
  **不自动覆盖**工作目录；尤其是 `python.exe`、`node.exe` 的目录可能不是项目目录。
- 裸命令如 `python`、`node` 不显示目录建议，允许用户手动填写项目工作目录。
- 保存前本地检查 Windows 绝对路径语法，然后通过已鉴权的后端 API
  **在运行 PerchFlow 的电脑上**检查目录是否真实存在、是不是文件夹。
  浏览器本身不具有任意读取本地文件系统的能力，不能只靠前端做虚假的存在性校验。
- 校验失败时直接在输入框下方展示中文或英文提示，禁止保存；常见失败：路径非绝对、
  目录不存在、输入了文件路径、无权限或无法连接服务。
- 工作目录为空时沿用已存在的运行规则：含路径的 EXE 命令可使用其父目录；
  裸命令通常继承进程管理器的工作目录。填写明确的 `cwd` 总是优先。
- 修改运行中的进程工作目录，会通过现有 supervisor 重启进程后生效。
  **仅修改中文别名**仍走独立 alias API，不重启运行中的进程。
- 短屏幕时弹窗内部滚动；主页面、原进程长表格、日志、任务页面保持原样。

## Rust 后端的最小安全补充

这次没有新增执行机制，只补全两项用于 UI 的能力：

1. `POST /api/processes/validate-cwd`，请求体 `{"cwd":"D:\\Projects\\MyApp"}`。
   现有 Bearer Token 鉴权，返回 `{"ok":true}` 或
   `{"ok":false,"error":"cwd_not_found"}` 等稳定错误码。
2. `PUT /api/processes/{name}/cwd`，请求体 `{"cwd":"D:\\Projects\\MyApp"}`，
   `null` 清空。只更新并持久化此进程的 `cwd`；保留其 `env`、`restart`、
   中文别名等其他配置。运行中更改工作目录会重启以生效，未变更不会重启。

完整的 `PUT /api/processes/{name}` 仍支持所有已有字段，并在客户端未提交隐藏的
`env` / `restart` 时保留 TOML 的原有值，避免 UI 编辑将其意外重置。

实际启动前，`spawn_process` 还会检查已配置的目录是否仍存在，用于处理
「保存后目录被删除或移动」以及手动编辑 TOML 等情况。不会检查、访问或修改项目数据。

以上接口不对公网开放；与现有接口一样仅在本机 PerchFlow 鉴权后可使用。

## 开发测试

```powershell
python -m unittest discover -s dev -p test_*.py -q
python dev/verify_process_cwd_browser.py
python dev/verify_process_alias_browser.py
python dev/verify_process_table_browser.py
python dev/verify_process_detail_browser.py
python dev/verify_tasks_table_browser.py
python dev/verify_topbar_controls_browser.py
python dev/verify_nav_stability.py
node --check frontend/assets/processes.js
node --check frontend/assets/process-cwd.js
```

`dev/test_process_cwd.py` 使用系统临时目录测试有效文件夹、文件路径、缺失目录、相对路径。
`dev/verify_process_cwd_browser.py` 使用 Edge 和 `9011` Mock 检查界面及 CRUD，
并在临时路径中做实际目录检查，测试结束会销毁自己的临时目录。
`tests/api_test.rs` 已补充 Rust 真实 API 的鉴权／校验、TOML 保存、
环境变量与重启策略保留等集成测试；需要等后续 Windows Rust 构建验证。

**生产安全**：本阶段只修改开发分支和 `9011` Mock。
正式安装 `D:\Downloads\perch-v0.1.2` 和之前发布的 Portable ZIP 不替换；
用户验收页面后，再编译正式 EXE 并按已有安全备份流程升级。
