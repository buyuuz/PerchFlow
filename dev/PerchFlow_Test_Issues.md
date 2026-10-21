# PerchFlow 本地测试问题清单（C2）

> **记录模式**：仅记录问题及源码排查结果，未经用户明确说“可以开始修复”，不得修改功能代码、测试代码、CI 工作流、重新构建或部署。 
>
> 测试环境：C2，独立 PerchFlow Project Hub RC。用户运行的真实业务脚本不在此仓库存储。
>
> GitHub 仓库当前禁用了 Issues，故以本文档为持续问题跟踪入口。

## 状态汇总

| 编号 | 优先级 | 状态 | 问题 | 首次发现 |
| --- | --- | --- | --- | --- |
| PF-001 | P1 | 已记录／待复核／未修复 | 任务卡显示「运行中」，点停止却提示 task is not running | 2026-10-21 |

---

## PF-001 — 「停止」报 task is not running，但任务卡仍显示「运行中」

- **首次反馈**：2026-10-21，C2 用户提供 PerchFlow 项目页截图。
- **影响项目**：`Photos_AuUp`，任务名 `Photo_AutoPublisher`。
- **截图证据**：用户对话附图 `image(20261021-024138).png`（未上传到仓库）。
- **环境**：独立测试版，参考源码 `feature/project-hub-live-runs`；Cron 卡片状态为「已暂停」。
- **状态**：**仅调查并登记，未尝试修复或复现真实发布脚本。**

### 用户可见现象

任务卡显示 `状态：运行中 · Cron：已暂停`；「运行一次」为灰色、「停止」可点击。用户点击「停止」后，项目页面顶部显示：

```text
process error: task Photo_AutoPublisher is not running
```

界面同时仍显示「运行中」，构成明显的前后端状态不一致。

### 排查结论（源码支持）

1. 项目页 [`monitorTaskRun()`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/projects.js#L374-L395) 每秒轮询一次运行状态，但**最多 90 次**（约 90 秒）。如果脚本超过 90 秒才结束，任务卡可能永久停留在旧的「运行中」显示，直到手动刷新页面/状态。其他入口触发的任务也未必启动项目页这一短时轮询。
2. 任务卡的状态与停止按钮来自此前加载的 [`taskViews`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/projects.js#L246-L273)。[`operateProjectTask()`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/projects.js#L336-L369) 在 Stop 失败时显示错误，但**没有自动重新核对任务状态**。
3. [`POST /api/tasks/:name/stop`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/web/api_task.rs#L173-L178) 直接调用 [`scheduler.cancel_task()`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/core/scheduler.rs#L140-L149)，只有任务仍位于后台 `running` 映射中才能取消；否则抛出截图中的确切错误。自然完成的任务会从该映射移除。
4. **最新「执行详情」2 秒自动刷新修复主要覆盖运行历史与用户主动打开的日志，不能等同于任务卡持续刷新。** 因此该问题仍有出现条件。

### 根因推断与不确定性

**可能性最高**：真实脚本处理多项业务而运行超过 90 秒；后台执行完成并清理活动任务映射，但前端仍保留旧「运行中」，此时再点击 Stop 会返回 `not running`。

**次要可能性**：任务自然完成与用户点击 Stop 几乎同时发生，产生竞态；前端仍使用操作前的旧状态。

**尚未验证**：截图时真实 Python 进程是否已经结束、是否存在脱离父进程的子进程，以及该次运行历史的最终状态。**不能据此断言停止按钮未能停止一个真实运行中的 Python 进程。**

### 预期行为

- 任务卡显示的运行状态应与后端实际状态及时同步，长任务（大于 90 秒）也一样。
- 对已自然结束的任务不应显示可操作的 Stop；如果操作时刚结束，应给出「任务已结束」等清楚提示，并自动同步卡片，而非持续显示运行中。
- 不改变业务脚本执行时机、参数或已创建的排期。

### 后续修复待办（现在**禁止执行**）

1. 任务卡在项目页可见时持续、节流同步运行状态，避免 90 秒后停止刷新。
2. 与后台 `running` 映射、运行历史保持一致，修正 Stop 结束竞态的错误反馈。
3. Stop 返回 `not running` 时获取最新任务状态，并同步按钮可用性与提示。
4. 添加超 90 秒、刚结束即点击 Stop、跨页面启动任务、手动刷新/关闭详情等回归用例。
5. 研究后台已退出但可能残存浏览器子进程的边界；需要实际可控的模拟脚本验证，不能直接对真实 TikTok 业务操作。

### 下次现场复核（仅只读）

- 出现提示后点击「刷新状态」，观察状态是否更新到「成功／失败／取消／超时」。
- 查看「执行详情」最近一次记录的 `run_id`、结束时间、执行状态、退出码。
- 如仍显示运行中但 Stop 找不到任务，后续再核对是否属于运行时状态一致性问题，而非仅界面缓存问题。
- **不要因页面未刷新而重新点击「运行一次」**，避免重复发布或重复排期。

---
后续问题按 PF-002、PF-003… 继续追加；只有用户明确授权后才统一修复。
