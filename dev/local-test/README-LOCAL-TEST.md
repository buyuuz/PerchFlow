# PerchFlow Project Hub — PF-005～PF-009 界面验收测试版

这个测试包来自 `feature/project-hub-safe-log-naming` 的 Windows 64 位开发候选版，包含此前 Project Hub 功能、项目页重构以及 Cron 与脚本管理的解耦。**不是正式升级包，不要替换生产 PerchFlow。**

## 最简单的使用方法

1. 下载 GitHub Actions 中名为 `perchflow-safe-log-naming-windows-test` 的构建产物，解压外层 ZIP，再解压其中的 `PerchFlow-Safe-Log-Naming-Test-Win64.zip`。
2. 将 `PerchFlow-Safe-Log-Naming-Test-Win64` 整个文件夹放到新位置，例如 `D:\\Downloads\\PerchFlow-RC-Test`，**不要**解压到原来的 PerchFlow 安装目录。
3. 双击 **Start-Test.cmd**。第一次启动会生成独立 `perch.toml`、随机回环端口与随机登录令牌；也会在浏览器中打开该本地地址。
4. 在启动窗口复制显示的登录令牌，登录网页，然后打开 **项目 / Projects** 页面。
5. 查看已创建的 `RC Sandbox Demo` 项目。如果能检测到 Python，将自动出现 `rc_mock_python` 任务，**完全没有 Cron 定时计划**。先点击“运行一次”进行本地安全测试；在「定时任务」页面应当看不到这个示例脚本。
6. 看看执行状态、输出文件和文件变化是否正常：`demo/results.txt` 与 `demo/output/report.csv` 应在运行后创建，运行日志在 `task-logs`。
7. 完成测试后双击 **Stop-Test.cmd** 停止**该测试包自身的后台 PerchFlow 进程**。停止前请确保没有任务正在执行。

## 已有 RC 测试目录的升级方法（C2）

如果之前已经在旧测试包里配置好 Photos_AuUp，请**不要解压新版覆盖整个目录**。

1. 确保真实 Python 脚本已结束，再双击旧测试目录的 Stop-Test.cmd。
2. 先备份该目录，特别是 perch.toml 和 task-logs 文件夹。
3. 从新版 ZIP 里只提取 perch.exe，复制替换**旧测试目录中的** perch.exe。
4. 沿用旧的 Start-Test.cmd 和 perch.toml 启动，验证原有项目、Python 路径与历史仍在。
5. 首次只运行 RC Sandbox Demo 或其他纯模拟脚本，**不要重跑已执行的 TikTok 发布任务**。

## PF-001～PF-004 新功能

- **多脚本整行列表**：选中项目后，每个已登记 Python 任务占一行，点击脚本名或「详情 ↗」查看独立详情。项目文件夹中其他 .py 不会被自动执行；需要通过「+ Python 任务」分别确认添加。
- **运行指标**：详情页有实时 PID、CPU、内存、运行时间、退出码；正在执行时约每 2 秒更新。
- **状态一致性**：项目任务卡超过 90 秒仍自动刷新，脚本自然结束时「停止」不会持续可用。
- **日志三次保留**：每个任务从本次的 0 行开始显示，每次使用独立 run_id 日志。真实执行第四次时，PerchFlow 自动清理该任务最旧日志，仅保留最近三份；历史记录摘要仍在，其他任务日志与业务数据不受影响。**升级或仅打开页面不会自动清理旧日志**。日志可能含敏感信息，请仅在需要时点击「查看日志」。
- **文件候选发现**：详情页最多读取 128 KiB 的已登记 Python 源码，推断可能引用的 Excel、TXT、CSV 等文档名；不是完整依赖分析，动态路径可能漏检，不读取文档正文。
- **文件打开**：需用户点击并确认，才能尝试在运行 PerchFlow 的本机 Windows 默认程序中打开文件。若 PerchFlow 在 Windows Service Session 0 运行，无法保证可见的应用窗口；建议仅在 C2 登录的交互桌面测试。

## 测试功能建议

- 项目和 Python 任务能否创建、编辑、查看。
- 任务“运行一次”后状态、`run_id`、日志、输出文件变化能否显示。
- 删除 `demo/input.txt` 后，输入预检是否能阻止执行；恢复它后能否正常执行。
- 再次运行是否记录“修改已有文件”。
- 仅对虚构示例任务测试添加/移除 Cron；默认暂停，无需启用自动调度。
- 中文/英文切换、页面导航和移动宽度下的响应布局。

## 安全边界

- 启动程序始终使用 `perch.exe run` 的**前台运行模式（通过独立隐藏进程启动）**，不会调用 `install`、`uninstall`、`start`、`stop` 服务命令，不需要管理员权限。
- 首次配置包含 **0 个自启动进程，0 个已启用的 Cron 任务**。
- Web UI 仅绑定到 `127.0.0.1`，使用随机端口和随机令牌。登录令牌只保存在本地 `perch.toml` 中，启动窗口也会显示。
- Demo Python 脚本只读 `demo/input.txt`，只写本文件夹下的 `demo/results.txt` 与 `demo/output/report.csv`，不连接网络，不接触真实账号或第三方 API。
- 测试版管理器本身具有执行任意用户所配置命令的能力。**不要在未经确认前导入真实运营任务**。如果后来手动启用了真实任务，管理器确实会执行它们。
- 停止程序会尝试只停止记录 PID 且可执行文件路径匹配此文件夹的 `perch.exe`，不会按进程名称全局结束其他 PerchFlow。
- 别把测试版目录中的登录令牌、日志或你后来添加的真实文件上传到公共仓库。
- 如果 Python 没有在系统 PATH 中找到，Demo 项目仍可浏览，但不会自动配置 Python 任务；可在项目页手动指定解释器的绝对路径。

## 常见问题

**启动没反应？** 检查是否完整解压、Windows Defender 是否拦截未签名 EXE，或 `logs/` 中的报错。不要为了测试关闭安全防护。

**端口被占用？** 测试版会拒绝占用其他程序的端口，不会停止别的服务。关闭本测试版后，备份或删除测试版自身的 `perch.toml` 即可在下次启动重新随机选端口（这会重置测试配置）。不要动原版配置。

**怎样重新开始？** 先 `Stop-Test.cmd`，仅删除本测试文件夹下的 `perch.toml`、`test-instance.json`、`logs`、`task-logs` 与 Demo 产出，然后重新 `Start-Test.cmd`。不要把这套操作用于原来的安装目录。

**Windows 服务与可视化浏览器是否已测试？** CI 对临时 Windows 服务和后台 Python 模拟已通过，但你自己电脑上的系统权限、Python 环境、GUI 浏览器/RPA 仍需单独测试；本包不会自动安装服务。

**版本来源**：`buyuuz/PerchFlow`，独立分支 `feature/project-hub-safe-log-naming`。候选版代码尚未合并正式 `main`。

## 需要反馈什么

如发现问题，请提供：具体点击步骤、错误提示、页面截图、`perch.toml` 中**去掉 token 后**的相关配置，以及必要的**脱敏日志**。不要直接发送完整令牌、邮箱密码或其他账号资料。

## 本版新增验收（Cron 可选）

1. 新建项目时不创建 Cron；新增 Python 脚本不再要求填写 Cron。
2. 初次打开测试包时，“RC Sandbox Demo”中的 rc_mock_python 默认是手动任务。它应存在于“项目”页面，但不出现在“定时任务”页面。
3. 在脚本详情中选择“添加/修改计划”，保存后默认暂停，任务才出现在“定时任务”页面。
4. “移除计划”只删除 Cron 配置，不删除脚本、关联文件、运行历史。
5. 所有验证只针对 demo 的 mock_worker.py。不要为真实 TikTok 发布脚本启用定时任务，也不要将此 ZIP 覆盖现有生产目录。

## PF-005～PF-009 用户界面验收

- PF-005/006：项目页字体与“进程”“定时任务”一致，页面标题 17px；表格与操作按钮采用相近的字体、行高及间距。
- PF-007：在桌面宽度下，项目列表纵向固定在左边，选中项目的脚本表位于右边；项目很多时只滚动左侧列表。
- PF-008：在常见桌面窗口（1600×900、1366×768、1280×720）中，Python 脚本详情页不发生整页纵向滚动。日志、历史和文件很多时只在各自区域内部滚动。
- PF-009：脚本标题、图标、操作按钮和五个运行状态卡片缩小纵向高度，给下面的详情信息留出更大空间。
- 窄屏或窗口高度极小的环境，允许响应式降级为正常页面滚动，确保内容和操作不会被永久遮挡。
- 此版本通过 mock Edge 多分辨率/操作检查，构建/模拟脚本均不访问真实业务数据。首次在 C2 使用时仍建议只打开 RC Sandbox Demo。

## PF-010 / PF-011 layout and terminal reuse

- PF-010: Projects view uses 1:3 left project navigator/right script list split on desktop; side gutters match the Scheduled Tasks page (16px at 1600px CSS viewport).
- PF-011: Script detail allocates approx 27% / 40% / 33% of sidebar height to Run History / Linked Files / Script Information; excess data scrolls inside each panel.
- PF-011: Script log panel reuses the Process Detail terminal HTML/CSS classes and existing process-detail.css without reusing process-specific API calls. The Python log endpoint and click-to-read security behavior are unchanged.
- PF-011: Terminal has colorized log lines, clear view, last 100/500/1000 lines and a live polling switch.
- The left script log area no longer duplicates Run History and Linked Files tabs.

## PF-012..017 UI improvements

- Opening a discovered file from Script Detail is one click; successful opens no longer show a session warning. Server-side known-file allowlist and loopback-only restrictions remain.
- The latest run log loads automatically if a history record exists. Clicking a historical run immediately loads that run's log. No history causes no log fetch.
- Script Settings opens a local dialog (interpreter/script path/working directory/one argument per line/timeout); edits use a project-scoped API that preserves task identity, Cron enabled/schedule, and history.
- The Script Info edit link, exit-code/CPU/memory fields, and the three-dots shortcut in project script rows were removed. Existing task data remain unchanged.
- The compact single-viewport Script Detail, terminal log styling and per-panel scrolling remain in place.

## Collision-resistant task log storage

- New per-run output goes to task-logs/<sha256-of-exact-UTF8-task-name>/<run_id>.log; output-file metadata uses <run_id>.files.json alongside it. Unicode labels, punctuation and very long names cannot accidentally collapse to the same underscored filename.
- The existing perch.toml, task identity, project membership, schedules and task-logs/history.jsonl stay unchanged.
- Unique legacy paths are still readable: task-logs/<legacy_sanitized_task_name>-<run_id>.log (and .files.json). A legacy filename prefix shared by multiple currently registered tasks is ambiguous: leave it untouched and DO NOT associate it with the wrong task.
- Log retention counts the direct-hash directory, previously generated v2 hash files, and unambiguous legacy per-run logs together; keep the latest three unambiguous log bodies per task; old ambiguous files are deliberately left untouched to avoid data loss. Per-run .files.json metadata and the JSONL history follow their pre-existing separate retention behavior.
- All operations run in the isolated test package, not the active C2 Windows service.

- The v2 directory is no longer created for new runs. Previously generated task-logs/v2/<hash>/ logs and .files.json records remain readable; they are not moved or mass deleted. A pre-existing empty v2 folder, if any, is left untouched for safety.
