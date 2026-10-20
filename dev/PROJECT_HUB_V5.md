# PerchFlow Project Hub V5 — 项目页面内的任务执行与定时管理

## 目标
在已经通过 V4 隔离测试的项目管理页直接操作任务与查看单次运行详情，而不再频繁切换到 /tasks。

## V5 功能
- 每条已有项目任务显示状态、当前 Cron 启停、Cron 表达式。
- 项目页新增「运行一次」「停止」「启用定时／暂停定时」「执行详情」「任务配置」按钮。
- 「运行一次」复用已有 POST /api/tasks/:name/run，不论 Cron 是否启用都执行；执行前强制二次确认。
- 「停止」复用已有 POST /api/tasks/:name/stop，按原调度器取消正在运行的任务；需要确认。不会删除文件。
- 「启用／暂停定时」新增 PUT /api/projects/:name/tasks/:task/schedule（JSON enabled: boolean）。
- 新接口校验任务确属该项目；只允许改变任务 enabled 字段，用原来的原子 perch.toml 保存与 Scheduler.reload，不能改 cmd/args/cwd/任务名字/文件路径。
- 暂停 Cron 只影响未来触发，不会中断已经运行的任务；取消运行需要单独使用「停止」。启用 Cron 必须确认，因为会开始根据规则触发真实脚本。
- 手动运行后页面按需短时轮询当前任务状态，最多 90 次、每秒一次；不建立额外后台进程，其他场景有手动「刷新状态」。
- 同页运行历史可切换每个 run_id，查看开始结束时间、耗时、状态、退出码、输入预检、产出文件变化，并在明确点击后才加载可能有敏感信息的原始日志。
- 不复制、移动、编辑、删除业务脚本或输入输出文件。保留已有 V3/V4 限额和 metadata-only 文件快照。

## 安全原则
- 新任务由项目创建时仍默认 disabled，不会自动启动。
- 用户必须确认启用定时或运行一次；这些 UI 控制不是纯显示，在未来部署后确实会触发配置的任务。
- 后端路由仍由既有 /api Bearer Token 保护，未经鉴权不允许修改调度状态。
- 运行日志原来就可能含敏感信息；依然只有手动点击时才加载，不能称为已脱敏。
- 手动任务运行与 Cron 同属一个 Scheduler，原有去重、取消、超时和运行历史行为继续沿用。
- 当前开发只在 feature/project-hub-v5，不触碰 main、用户电脑、Windows Task Scheduler、Daily_Check、Firecrawl 登录或 TikTok 业务。

## GitHub Windows 测试
- Rust API：只允许对项目所属任务启停 Cron，拒绝越界、未鉴权，检查持久化、只修改 enabled、重复调用幂等。
- Rust API：以临时 powershell Start-Sleep 为假任务，禁用 Cron 状态下可手动启动；在运行中启停 Cron 不强制停止；显式停止后历史记录为 cancelled。
- 模拟 Edge 页面：确认弹窗，Cron 启停，手动运行及停止，执行详情与日志按需读取；桌面与移动宽度。
- 现有 cargo test、Windows release build、JS syntax、隔离调度 EXE 与 SCM 临时服务完整回归。

## 仍待后续
- 单任务独立依赖清单；任务参数和 Cron 的项目页内编辑（本版仍进入 /tasks）。
- 持续运行超过短时轮询期限的实时更新、批量任务控制、依赖 DAG、多任务编排。
- 全量视觉验收和正式生产部署；所有真实 Windows 任务仍保持原样。
