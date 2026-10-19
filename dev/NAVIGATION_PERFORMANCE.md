# PerchFlow 两页切换性能排查与修复

日期：2026-10-19
工作分支：feature/frontend-dev-preview

## 原因

当前 `/processes` 与 `/tasks` 是两个独立 HTML 页面，导航会重建 DOM、执行 JS 并重新读取样式和字体。开发预览服务器过去对**所有**资源（包括字体）发送 `Cache-Control: no-store, max-age=0`，导致每次切换都会重新下载字体。

`frontend/assets/fonts/material-symbols-outlined.woff2` 的原始体积是 **3,130,160 字节**。另外 5 个字体文件总计约 111 KB。重复网络传输与字体渲染让页面切换显得拖沓。

## 优化

- `dev/preview_server.py`：仅对 `.woff2` 添加 `Cache-Control: private, max-age=86400`（24小时），HTML、CSS、JS、API 继续 `no-store, max-age=0`，保留改前端源码后手动刷新即可更新的开发特性。
- `src/web/assets.rs`：内嵌资源服务对字体使用相同缓存规则，对其他文件发送 `no-cache`，便于后续正式 EXE 更新时重新校验非字体资源。**这项 Rust 源码变更要等 UI 定稿后通过 GitHub Actions 编译才会进入正式 EXE**。
- 没有引入 SPA 框架或额外依赖，没有变更 Perch 进程 API。

## Edge 测试（同一公司电脑，9011 Mock 预览）

测试命令：

```powershell
python dev/profile_navigation.py
python -m unittest discover -s dev -p test_*.py -v
python dev/verify_process_table_browser.py
```

使用 Edge Playwright 循环点击「进程」与「定时任务」六次，每次等到 Mock 进程或任务行出现。

| 指标 | 修复前（典型值） | 修复后（典型值） |
| --- | --- | --- |
| 往返单次导航（点击至内容可用） | 约 280–420ms | 约 94–180ms |
| Navigation Timing load event | 约 200–370ms | 约 41–106ms |
| 连续切换字体 HTTP 网络传输 | 每次约 3.24MB | 0 字节（浏览器缓存） |

说明：这些是几次本机实测，不保证不同电脑、缓存状态、负载都得到相同速度；首次打开页面仍需下载字体。使用 Edge DevTools 的 **Disable cache** 会使缓存优化失效。

## 兼容

- 开发预览 9011 继续只在本机运行，不触碰正式 9002/9877 或已有托管项目。
- 20 项 unittest 通过，另用 Edge 验证桌面／窄屏布局与搜索、排序、复制、编辑、删除、模拟启停及日志跳转。
- Font 缓存 24 小时。如果未来直接替换了同路径字体文件，请使用 Ctrl+F5 强制刷新。
- 由于目前仍然是多页面架构，页面 DOM 会重建。若将来希望像桌面应用一样无刷新切换，可再评估共享导航、局部路由或 SPA，但本次不引入复杂度。
