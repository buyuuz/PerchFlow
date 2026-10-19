# PerchFlow 定时任务长条表格（开发预览）

更新时间：2026-10-19。分支：`feature/frontend-dev-preview`。

## 本轮目标

保持现有深色、青蓝色的管理器设计，将旧的桌面行卡片／移动端卡片改为与“进程管理”一致的整行列表式表格，并保持相同顶部导航与标题比例。

## 字段

每个定时任务一行，列包含：

1. 选择框（只负责选中／高亮／取消选择，**不执行批量操作**）
2. 任务名称、项目图标
3. 已启用／已停用状态徽标
4. Cron 表达式（原始表达式可完整悬停查看）
5. 启动命令（可复制）
6. 上次运行时间（浏览器本地时区）
7. 上次运行结果（成功／失败／尚未运行；使用 Rust API 的退出码）
8. 操作（立即运行、启用／停用、编辑、删除）

注意：当前 Rust API `/api/tasks` **没有返回下次运行时间**，这版不会填入推测值。若未来需要，需要设计可靠的调度器数据字段再单独增加。

## 交互

- 搜索匹配任务名称、Cron 表达式、命令和当前状态
- 点击名称、状态、Cron、上次运行和运行结果等表头可切换升序／降序
- 编辑、新建、删除、立即运行、切换启用状态直接复用现有 `/api/tasks` 路由，未更改 Rust API
- 保留原有中文、英文、西班牙语、日语、葡萄牙语支持
- 3 秒刷新会保留搜索和排序
- 73px 桌面行高、68px 小屏行高；窄屏仅在表格区域横向滚动
- Mock 数据添加了四种典型状态，所有点击仅更改预览内存，不会执行真实脚本或触碰生产任务

## 文件

- `frontend/tasks.html`
- `frontend/assets/tasks-table.css`（复用 `process-table.css` 视觉原语）
- `frontend/assets/tasks.js`
- `dev/preview_server.py`（四条模拟任务）
- `dev/test_tasks_table.py`（Python 标准库源代码回归测试）
- `dev/verify_tasks_table_browser.py`（Edge Playwright 实际操作和布局测试）

## 检查

```powershell
python -m unittest discover -s dev -p test_*.py -v
python dev/verify_tasks_table_browser.py
node --check frontend/assets/tasks.js
```

浏览器查看：http://127.0.0.1:9011/tasks

所有测试使用 Mock 模式；直到用户确认 UI 定稿前，不创建 `ui-build-*` 标签，不通过 GitHub Actions 编译正式版本，也不替换已部署的 `perch.exe`。
