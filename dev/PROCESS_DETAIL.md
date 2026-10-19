# PerchFlow · 进程详情页整体美化

更新时间：2026-10-19
分支：`feature/frontend-dev-preview`。
范围：只修改 `/process-detail?name=...` 前端展示与浏览器交互，不调整 Rust API、后台托管、端口或正式 EXE。

## 设计原则

- 跟「进程管理」及「定时任务」统一深色背景、青蓝色强调、细线边框、8–12px 圆角和紧凑工具栏。
- 不再保留旧版巨大的顶部标题和分散的操作按钮。
- **桌面端一屏查看**：不滚动整个页面，尽可能保留大尺寸日志窗口；日志内容独立滚动。
- **小屏自然滚动**：标题和工具按钮分行、资源指标变为两列，避免整个网页水平溢出或截断日志。
- 对状态/操作保持真实语义：Running 时禁用「启动」，Stopped/Crashed 时禁用「停止」；「重启」仍按原有 API 提供。
- 延续此前决定：语言选择仅中文和 English，沿用共享 `topbar-controls.css/js`。

## 主要变化

1. 顶部：返回按钮、路径导航（进程管理 / 进程详情）、进程名称、状态圆点标签；右侧统一 32px 高语言、停止、重启、启动控件。
2. 信息面板：PID、CPU、MEM、运行时间在一行，分隔线提升可读性；启动命令单独一条并新增一键复制。
3. 日志工作区：16px 标题、日志行数指示、统一32px的清屏/历史行数/实时开关；终端头部标记 `stdout / {name}`、底栏实时模式状态、12px 字体与 1.75 行高，日志包含 INFO/WARN/ERROR 时用不同文字颜色。
4. 交互：保留原有 `/api/processes/{name}`、`/api/processes/{name}/{start|stop|restart}`、日志历史及 SSE `/api/logs/stream/{name}`；保留清屏只清空屏幕、实时增量跟随和上翻后停止自动滚动的行为。
5. 语言：新增详情页专属 zh/en 文本及随语言变化重绘动态进程状态、日志行数和空状态。
6. 安全：日志文本仍通过 HTML 转义输出；浏览器测试先验证 `/_dev/status` 返回 `mock`，不允许在真实进程服务上执行变更。

## 修改文件

- `frontend/process-detail.html`
- `frontend/assets/process-detail.css`（独立样式）
- `frontend/assets/process-detail.js`
- `frontend/assets/i18n.js`
- `dev/test_process_detail.py`
- `dev/verify_process_detail_browser.py`

## 验证

```powershell
node --check frontend/assets/process-detail.js
python -m unittest discover -s dev -p test_*.py -v
python dev/verify_process_detail_browser.py
python dev/verify_process_table_browser.py
python dev/verify_tasks_table_browser.py
python dev/verify_topbar_controls_browser.py
python dev/verify_nav_stability.py
```

浏览器实测 1440×900、1280×800、768×850、390×844、320×750。新页面 CSS 和 JS 热预览地址：

`http://127.0.0.1:9011/process-detail?name=MailHub%20(DEMO)`

本阶段不触发 Actions、不更新正式 `perch.exe`，也不影响 MailHub。等 UI 由用户确认定稿后统一编译正式 EXE。
