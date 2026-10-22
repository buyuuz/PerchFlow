# Project Hub 发布候选准备：Python 项目隔离兼容性验证

## 本阶段目标

将 V1–V5 经视觉与 API 验收的候选版本，在 GitHub 托管的**一次性 Windows 环境**中，使用**真正编译出的 perch.exe + Python 解释器**测试完整项目执行链路。

这是模拟项目的集成测试，**不是**对用户电脑、Firecrawl、Daily_Check 或任意真实业务的兼容性结论。

## 测试范围

新入口：`dev/verify_project_runtime_compat.py`；CI 工作流：`.github/workflows/project-rc-compat-ci.yml`。

全程使用临时文件夹和仅标准库的虚构 Python 脚本。真实 Python 负责创建虚构 TXT、CSV、PNG 字节文件。PerchFlow 只记录已登记路径的元数据。所有 HTTP 只发向本地回环地址；不接入 TikTok、Firecrawl、代理、账号或第三方服务。

十一项验证项：

1. 编译的 perch.exe 从全新临时目录启动。
2. 项目页对应的 Python 任务 API 正确保存解释器、脚本、参数和工作目录，并强制 `enabled=false`。
3. 没有必要输入文件时执行被预检阻止，不启动 Python。
4. 补齐虚拟输入后真实执行 Python，正确使用 CWD、捕获 stdout、生成文件。
5. 固定输出与输出目录自动发现按 `run_id` 记录创建变化，管理端响应不泄露虚构文件正文。
6. 再次运行同一脚本，按 `run_id` 记录修改变化。
7. Python 返回非零退出码，任务历史/日志一致。
8. Python 超时，被调度器终止并记录。
9. 手动停止正在运行的 Python，记录取消。
10. PerchFlow EXE 在隔离目录重启后恢复任务历史与文件变化记录。
11. 所有副作用均限制在 GitHub 临时目录，真实生产项目没有被调用。

所有任务的 Cron 开关保持关闭，不触发真实计划发布。

## 生产级迁移之前的手动验收关卡

**不因 CI 全绿而自动合并或接管脚本。**

1. 先明确目标机器、执行会话（Windows 服务 Session 0 或已登录桌面）、Python 与虚拟环境、工作目录、文件/磁盘权限、是否需要交互式浏览器。
2. 备份原 `perch.toml` 与 Windows 计划任务设置；保持原有任务和数据路径不变。
3. 选取一条可安全重复、不会上传/发布/修改第三方服务状态的脚本，人工确认允许在隔离设备上试运行。
4. 该脚本先以禁用 Cron 方式登记，检查输入就绪，再手动运行；确认退出码、stdout/stderr、输出清单、权限和停止/超时行为。
5. **绝不同时启用原 Windows 计划任务与 PerchFlow Cron**，避免重复发布、发邮件、覆盖表格等副作用。需要切换时先人工确认，保留回退路线。
6. 对 Selenium/Playwright headed、可视 RPA、鼠标/键盘/剪贴板、Chrome 登录态等脚本，另行在交互式 Windows 用户会话测试；GitHub Windows Runner 成功不代表 Windows 服务 Session 0 可用。
7. 测试失败应停止迁移并回退为原调度安排；不得自动修改原业务目录或接管真实任务。

## 发布候选阶段的结论口径

- GitHub Windows CI + 虚构 Python 项目通过 = **模拟后台 Python 链路初步兼容**。
- 真实 Python 项目、第三方网络登录、浏览器交互、用户 Windows 服务、跨机器执行均属于**未验证**。
- 不自动把用户的 TXT/API Key 上传到 CI，不会复制或归档运行输入。
- 不自动合并 `main`、部署或修改真实 Windows 服务/Task Scheduler。

## 验收记录

已通过的**隔离模拟验收**（全部使用 GitHub 托管的临时 Windows 环境）：

- **真实 PerchFlow EXE + Python 虚构项目**：11/11 通过。任务默认停用、输入阻止/恢复、stdout、模拟 TXT/CSV/PNG 输出、运行记录文件变化、重复修改、非零退出码、超时、停止、重启历史恢复均通过。[Run 39528201472](https://github.com/buyuuz/PerchFlow/actions/runs/39528201472)（测试代码提交 `6d32c37a5`）。
- **Rust 单元/API**：69 项通过（35+30+4），Release 构建成功，JavaScript 检查与既有 Windows EXE Cron/取消/超时/恢复模拟成功。[Run 39528306706](https://github.com/buyuuz/PerchFlow/actions/runs/39528306706)（候选代码提交 `aa910518e`）。
- **Windows SCM 服务回归**：通过。[Run 39528295357](https://github.com/buyuuz/PerchFlow/actions/runs/39528295357)。
- **项目浏览器桌面/移动端冒烟**：通过。[Run 39528301220](https://github.com/buyuuz/PerchFlow/actions/runs/39528301220)。
- **V1–V5 综合视觉/异常/交互验收**：通过。[Run 39528306768](https://github.com/buyuuz/PerchFlow/actions/runs/39528306768)。

**决策：隔离测试环境中的后台 Python 执行链路达到发布候选标准，但不批准自动合并、部署或接管真实任务。** 下一步应先对具体真实脚本做只读依赖与 Session 0/交互桌面兼容性审查，再由用户决定是否授权独立试运行或迁移。项目管理本身仍不会读取文件正文；虚构脚本自己读取测试数据的行为不代表 PerchFlow 读取用户数据。
