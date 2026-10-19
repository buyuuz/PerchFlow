# PerchFlow 顶栏控件（开发预览）

日期：2026-10-19。工作分支：`feature/frontend-dev-preview`。

## 设计调整

统一进程页面和定时任务页面顶部右侧三个控件：

| 控件 | 旧样式 | 新样式 |
|---|---|---|
| Connected 状态框 | 32px 高、独立的背景和圆角 | 32px 高，12px 字体，绿色小状态点、细边框、16px 圆角 |
| 添加进程／添加任务 | 36px 高、16px 字体、4px 圆角 | 32px 高、12.5px 字体、8px 圆角、收敛的青蓝渐变 |
| 语言选择 | Windows 原生 36px 高，14px 字体，4px 圆角 | 32px 高、12px 字体、8px 圆角、自定义深色下拉菜单 |

按钮间距统一为 10px（小屏 7px）；控件在顶部 56px 导航栏中居中。无大幅发光、文字位置不会因为悬停或选中状态改变。

## 语言菜单

- 通过 `frontend/assets/topbar-controls.js` 进行渐进增强：默认仍保留原生 `select[data-lang-select]`，加载 JS 后换成深色列表框；禁用 JS 时仍能看到原生选择框。
- 菜单有 CN 中文、EN English、ES Español、JP 日本語、BR Português 五个项目。当前选中项使用青色、浅底纹和勾选图标。
- 支持鼠标选择、点击外部关闭、Esc 关闭、方向键／Home／End 导航、回车确认和键盘焦点。
- 自定义控件在用户选中语言时更新原生 `select` 并触发正常的 `change` 事件；原有 `processes.js` 和 `tasks.js` 的表格多语言重绘流程不变。
- `i18n.js` 的 `setLang` 额外发出 `perchflow:language-changed` 事件，用于同步服务器主动指定语言时的菜单标签和选中状态。
- 弹出菜单使用绝对定位浮层，不参与文档布局，不会挤动搜索框或表格；弹出时覆盖页面部分内容属于正常下拉行为。
- <=767px 时隐藏 Connected 状态框，缩小语言控件和按钮水平内边距；<=370px 时隐藏品牌文字以避免手机端导航水平溢出。

## 主要文件

- `frontend/assets/topbar-controls.css`：统一控件、弹出层样式和响应式布局
- `frontend/assets/topbar-controls.js`：语言选择器键盘、点击、关闭、状态同步逻辑
- `frontend/processes.html` 和 `frontend/tasks.html`：使用同一套 CSS 与 JS、独立的添加弹窗功能
- `frontend/assets/i18n.js`：语言切换通知事件
- `dev/test_topbar_controls.py`：标准库回归测试
- `dev/verify_topbar_controls_browser.py`：Edge 真实布局和交互测试

## 验证

```powershell
node --check frontend/assets/topbar-controls.js
python -m unittest discover -s dev -p test_*.py -v
python dev/verify_topbar_controls_browser.py
python dev/verify_process_table_browser.py
python dev/verify_tasks_table_browser.py
python dev/verify_nav_stability.py
```

开发预览仍在 `http://127.0.0.1:9011`，模拟 API 不会操作任何真实项目。设计定稿前不推送 `ui-build-*` 标签或编译／替换正式 `perch.exe`。
