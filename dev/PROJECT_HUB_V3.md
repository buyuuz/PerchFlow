# PerchFlow Project Hub V3 — 输出目录自动发现

## 为什么增加此功能
V1/V2 只能追踪预先登记的具体文件。真实 Python 自动化可能生成文件名不固定的报表、图片、视频和临时结果。
本阶段新增显式登记输出目录，PerchFlow 在每次任务前后扫描限定范围的文件元数据，将变化关联到已有 run_id。

## 配置示例（仅虚拟路径与任务）
```toml
[[project]]
name = "Demo Output"
root = "D:\\Projects\\Demo"
tasks = ["mock_worker"]
inputs = ["worker.py"]
outputs = ["fixed-result.txt"]
output_dirs = ["reports", "output"]
```
在 /projects 页面编辑项目：输出目录每行一条项目相对路径。output_dirs 是可选字段，旧配置无需迁移。

## 实现设计与边界
- 只记录文件项目相对路径、类型、大小、修改时间、存在状态；不读取、复制、下载、编辑正文或计算哈希。
- 每项目最多 8 个输出目录；每个快照最多自动发现 512 个常规文件，最多访问 64 个目录；扫描输出目录根层与最多一层子目录。
- 此外仍可跟踪最多 128 个明确登记的 inputs/outputs 文件路径。
- 禁止 ..、绝对路径、盘符越界及通过文件/目录链接穿越根目录，进行 canonical 路径校验。
- 目录可在运行时创建，运行前后快照均会重新检测。
- 按 role+path 进行差异比较，不依赖目录枚举顺序。
- 目录读取失败或突破数量限制时标记 incomplete_scan=true，避免因不完整扫描而误报新增/删除。
- 每次结果仍存放在 PerchFlow 的 task-logs/<task>-<run_id>.files.json，JSON 最大 2 MiB，API 使用 Bearer 鉴权。
- 页面在执行记录展示自动发现标签与不完整扫描提示。
- 只观察运行时间区间的文件变化，不证明一定由该任务创建；并发程序可能同时写入。
- 扫描超出约定深度的文件不属于本次监测范围，并非自动全部覆盖。
- 文件名/路径也可能包含敏感信息，请只登记专用产出目录；原有任务 stdout/stderr 仍可能包含敏感值，本期不做旧日志脱敏。

## 测试与验证
- Rust 单元：显式文件兼容，目录新增/文件新增、修改/删除，数量限制，路径遍历拒绝，未读取正文。
- Rust API：在 GitHub 临时 Windows 目录用 MOCK 命令生成 reports/generated.xlsx，读取关联 run_id 的 files API，检查鉴权、路径验证及旧配置。
- Edge Mock UI：检查编辑输出目录、每次运行自动发现标签及桌面/移动交互。
- GitHub Windows Runner：cargo test、release build、既有任务和 SCM 隔离回归。

## 安全与尚未实现
- 仅 feature/project-hub-v3 分支，不修改 main，不触碰用户 Windows 计划任务或正式服务。
- 不执行 Daily_Check、Firecrawl 登录和 TikTok 发布，不使用真实账号密码或 Key。
- 不做自动清理、文件正文预览或下载、无限递归、桌面自动化执行器或正式迁移。
