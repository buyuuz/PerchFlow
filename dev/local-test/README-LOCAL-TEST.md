# PerchFlow Project Hub RC — Windows 本地测试版

这个测试包来自 `feature/project-hub-rc-compat` 的 V1–V5 项目管理功能，提供独立的 Windows 64 位可执行程序。**不是正式升级包，暂不替换你正在运行的 PerchFlow。**

## 最简单的使用方法

1. 下载 GitHub Actions 中名为 `perchflow-projecthub-rc-windows-local-test` 的构建产物，解压外层 ZIP，再解压其中的 `PerchFlow-ProjectHub-RC-Win64.zip`。
2. 将 `PerchFlow-ProjectHub-RC-Win64` 整个文件夹放到新位置，例如 `D:\\Downloads\\PerchFlow-RC-Test`，**不要**解压到原来的 PerchFlow 安装目录。
3. 双击 **Start-Test.cmd**。第一次启动会生成独立 `perch.toml`、随机回环端口与随机登录令牌；也会在浏览器中打开该本地地址。
4. 在启动窗口复制显示的登录令牌，登录网页，然后打开 **项目 / Projects** 页面。
5. 查看已创建的 `RC Sandbox Demo` 项目。如果能检测到 Python，将自动出现 `rc_mock_python` 任务，**定时调度默认禁用**。先点击“运行一次”进行本地安全测试。
6. 看看执行状态、输出文件和文件变化是否正常：`demo/results.txt` 与 `demo/output/report.csv` 应在运行后创建，运行日志在 `task-logs`。
7. 完成测试后双击 **Stop-Test.cmd** 停止**该测试包自身的后台 PerchFlow 进程**。停止前请确保没有任务正在执行。

## 测试功能建议

- 项目和 Python 任务能否创建、编辑、查看。
- 任务“运行一次”后状态、`run_id`、日志、输出文件变化能否显示。
- 删除 `demo/input.txt` 后，输入预检是否能阻止执行；恢复它后能否正常执行。
- 再次运行是否记录“修改已有文件”。
- 多次启停 Cron **仅对虚构示例任务测试**。若不需要，不必启用 Cron。
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

**版本来源**：`buyuuz/PerchFlow`，独立分支 `feature/project-hub-local-test`。候选版代码不是正式 `main`。

## 需要反馈什么

如发现问题，请提供：具体点击步骤、错误提示、页面截图、`perch.toml` 中**去掉 token 后**的相关配置，以及必要的**脱敏日志**。不要直接发送完整令牌、邮箱密码或其他账号资料。
