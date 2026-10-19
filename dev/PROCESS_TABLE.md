# 进程管理：长条表格布局

2026-10-19 由用户选择的长条表格效果图落地，仅更改 PerchFlow 的开发预览源码，不修改正式 PerchFlow EXE 或 `perch.toml`。

## 页面

- `frontend/processes.html`：进程列表使用独立长条表格（10 列）
- `frontend/assets/process-table.css`：桌面/窄屏视觉、状态颜色、按钮样式和滚动区域
- `frontend/assets/processes.js`：搜索、点击表头排序、行选择、命令复制、进程启停、编辑/新增/删除和日志入口
- `dev/preview_server.py`：模拟数据增加 `Data Collector (DEMO)` 一行，方便比对用户所选设计图

## 交互

1. 每个进程占一整行，列依次为：选择、进程名称、状态、PID、CPU、MEM、UPTIME、RESTARTS、启动命令、操作。
2. 点击可排序表头：首次升序、再次降序；进程搜索支持名称、命令、状态和 PID；3 秒轮询不会重置筛选或排序。
3. 每行保留启动 / 重启 / 停止 / 编辑 / 删除 / 日志六个原有操作入口，启动命令可以复制。
4. 左侧复选框目前用于选中与高亮行、查看选中数量和取消选择，**不会自动执行任何批量操作**。
5. 窄屏（例如 390px）在列表内部左右滚动，网页整体不横向溢出。
6. 状态徽标用运行中（绿）、已停止（灰）、已崩溃（红）和重启中（黄）区分；页面支持 zh/en/es/ja/pt 表头文本。

## 检查方式

```powershell
# Python 标准库测试；同时由 Actions 在正式编译时执行
python -m unittest discover -s dev -p test_*.py -v

# 可选：公司电脑已安装 Playwright + Edge 时进行实际界面点击测试
python dev/verify_process_table_browser.py
```

浏览器检查覆盖 1648px、1440px、390px 布局及搜索、排序、勾选、复制、编辑、新增、删除、启动、重启、停止、日志跳转、语言切换和自动刷新。预览地址：`http://127.0.0.1:9011/processes`。

Playwright 会在 `dev/screenshots/` 下保存本地截图；该目录已通过 `.gitignore` 排除。所有操作都指向 Mock 预览，不访问正式服务。设计定稿之前不推送 `ui-build-*` 标签。
