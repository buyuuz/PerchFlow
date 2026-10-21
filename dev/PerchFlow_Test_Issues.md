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
| PF-002 | P2（体验/视觉） | 已记录／源码对比完成／未修复 | 项目管理整页设计与进程／定时任务界面不统一 | 2026-10-21 |
| PF-003 | 需求（方案待确认） | 已记录／架构初审／未开发 | 一个项目多 Python 可执行任务：整行列表、独立运行详情、依赖文件识别与本地打开 | 2026-10-21 |
| PF-004 | 需求（日志保留策略） | 已记录／源码检查完成／未开发 | 每个 Python 脚本保留最近 3 次执行日志，每次执行从空白日志与 0 行开始 | 2026-10-21 |

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

## PF-002 — 项目管理页面与「进程／定时任务」的整体 UI 设计不一致

- **首次反馈**：2026-10-21，C2 用户在独立 RC 本地测试期间提出。
- **涉及页面**：`/projects` 对比 `/processes`、`/tasks`。
- **优先级**：P2（跨页面视觉/交互一致性，非阻断运行故障）；最终依用户确认再调整。
- **状态**：**已对照当前 GitHub 源码，只记录，不修改任何样式或功能。**

### 用户反映

从「进程」或「定时任务」切换到「项目管理」，整体视觉语言像是不同页面系统，没有统一的导航、按钮、布局、色彩、标题层级与信息密度。

### 代码对照确认的具体差异

| 方面 | 进程／定时任务（既有统一体系） | 项目管理（独立实现） |
| --- | --- | --- |
| 顶部导航 | 共用 `.pf-process-topnav`、`.pf-main-nav-link`，固定 56px，使用 `branding.css` 与 `topbar-controls.css` | 自定义 `.pf-project-header` / `.pf-project-nav`，粘性 60px，独立菜单间距和样式 |
| 导航高亮 / 主色 | 当前页下划线与主要按钮以青色系 `#46d6ed`／`#42d5ed` 为主 | 当前项与主要按钮采用淡蓝 `#7fa7ff`、`#a8c2ff` |
| 品牌和语言控件 | 共用 `.pf-header-brand`、顶部控件容器与语言选择交互 | `.pf-project-brand` 和原生 `select#project-language` |
| 页面主容器 | `max-w-[1600px]`，统一顶端留白及响应式边距 | `.pf-project-main` 最大宽度 1440px，独立 padding、留白 |
| 标题层级 | 页面主标题 17px（`pf-process-page-title`／`pf-management-page-title`） | 页面主标题 21px（`.pf-project-titlebar h1`） |
| 内容容器 | 进程／定时任务共用表格壳、深青灰渐变与 12px 圆角 | 项目两栏独立卡片容器，17px 圆角及其他蓝灰配色 |
| 操作按钮 | 进程／定时任务沿用共用顶部主按钮、表格行紧凑图标按钮 | 项目页独立 9px 圆角的蓝色主按钮、文字按钮；按钮字号和密度均另定义 |
| 移动端 | 进程／定时任务包含一致的底部主导航 | 项目管理单独将顶部导航换行；没有与其相同的移动端底部导航 |
| 对话框/表单 | 原页面使用同一批设计系统/模态框视觉规范 | 项目页另写 `.pf-project-overlay`、`.pf-project-form` 和输入框配色、间距 |

以上差异来自源代码，不等于用户已逐项确认所有元素都必须相同。其中**项目两栏结构可以保留**，问题主要是这套独立结构没有遵循整站设计规范。

### 相关源文件（仅用于追溯）

- [进程页面](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/processes.html)
- [定时任务页面](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/tasks.html)
- [项目管理页面](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/projects.html)
- [项目独立样式](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/projects.css)
- [共享品牌与导航样式](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/branding.css)
- [共享顶部控件样式](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/topbar-controls.css)
- [进程／任务共用表格样式](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/process-table.css)
- [原有设计规范](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/DESIGN.md)

### 用户诉求与后续修复方向（仅待办）

1. 以**当前「进程／定时任务」**作为已有视觉基准，统一主导航与活动态、顶部品牌、语言切换和移动端导航。
2. 统一字体层级、最大内容宽度、页面标题与副标题、间距节奏、颜色变量、圆角、按钮交互和弹窗样式。
3. **按 2026-10-21 用户补充需求修订：**保留“按项目分组”的概念，但项目内部改为像「进程」一样的**每个可执行 Python 任务一整行**，点击进入独立详情。旧版“保留项目详情卡片”的布局建议不再是已确认方案；具体见 PF-003。
4. 统一成功／失败／运行中的状态视觉及危险操作的提示层级。
5. 后续修复前最好对三页提供同屏对照设计，先由用户确认，再改组件、验收移动端与功能回归。

**限制**：当前为「记录阶段」，不得执行 UI 改造、修改功能代码、编译/打包或部署。等待用户明确说“可以开始修复”。

---

## PF-003 — 项目内多 Python 脚本管理、进程式独立详情与外部依赖文件识别/打开

- **反馈日期**：2026-10-21
- **类别**：产品需求补充 + 页面信息架构调整（**不是已经实现的功能，也不是已证实的故障**）
- **关联问题**：PF-002（项目管理与进程/定时任务 UI 统一）；PF-001（运行状态与停止操作一致性，详情页应一并解决）。
- **目标环境**：C2 的独立 PerchFlow RC；示例路径 `D:\Desktop\Aa\Tools\NIGHT-TTS\Photos_AuUp`。
- **状态**：**只登记与评估，尚无开发授权**。

### 用户原始意图（需求概要）

> 一个项目目录可能包含多个能独立执行的 Python 文件。像「进程」页面一样，把每个能执行的脚本呈现为一整行；点进某条就进入该 Python 任务的详情，包含 PID、运行时间、日志等信息。系统还应尽量发现该脚本引用的外部 Excel、TXT 等文件，提供直接打开文件的入口。

示例：

```text
项目：Photos_AuUp
目录：D:\Desktop\Aa\Tools\NIGHT-TTS\Photos_AuUp

┌───────────────────────────┬──────────┬────────┬──────────┬────────────┐
│ 可执行 Python 任务         │ 运行状态 │ PID    │ 运行时长 │ 操作       │
├───────────────────────────┼──────────┼────────┼──────────┼────────────┤
│ Photo_AutoPublisher.py    │ ...      │ ...    │ ...      │ 详情/运行/停止 │
│ sync_photos_to_excel.py   │ ...      │ ...    │ ...      │ 详情/运行/停止 │
└───────────────────────────┴──────────┴────────┴──────────┴────────────┘
```

以上是理想结构示例，不是真实任务运行数据。实际的多个任务需独立参数、解释器、工作目录、定时开关、运行状态与日志，不能共享一个“总任务”的 PID 或历史。

### UI/信息架构：进程式列表与独立详情

**项目页面**：可以先选择项目（项目名/根目录），再看到此项目全部**已登记的可执行入口**；每一条 Python 任务占一整行，遵循现有「进程」表格的字体、颜色、导航、行高、操作按钮和状态样式。

**每个脚本点击进入独立详情**，建议沿用现有「进程详情」页面的设计/交互：

- 任务名称、脚本绝对路径、Python 解释器与虚拟环境、工作目录、参数；Cron 表达式及开关；
- 运行中状态、子进程 PID（运行时）、持续时长、CPU 与内存占用（可取得时）；运行结束后保留运行历史的开始/结束时间、总耗时、退出码；
- 独立的实时 stdout/stderr 日志查看、历史运行列表、按 run_id 查看结果、手动运行/停止；
- 停止与当前 PID/运行状态必须保持一致，结束后按钮及时失效（关联 PF-001）；
- 进程与定时任务 UI 的整体风格保持一致（关联 PF-002）。

**注意**：一个目录下不是所有 `.py` 都应自动当成可执行入口，可能存在 `utils.py`、模块、配置生成器；可建议“发现候选脚本→用户勾选确认/添加”，默认不自动执行、不自动启用 Cron。

### 外部依赖文件识别与打开

用户希望查看 Python 脚本使用的 **Excel / TXT** 等外部文件，包含不在项目目录内、通过绝对路径或其他模块/配置文件引用的文件，并提供“直接打开”。

设计阶段拟区分三种信息，避免伪称百分百自动发现：

1. **代码推断**：静态分析 Python 的 `open()`、`Path()`、`openpyxl.load_workbook()`、`pandas.read_excel()` 等常见调用和字符串路径；对于动态拼接、环境变量、配置文件、其他模块函数、用户选择路径只能给出“候选”。
2. **已确认登记**：用户审核或手工加入的依赖文件（含外部目录）；显示真实路径、是否存在、文件类型、大小、修改时间、可能的读/写用途。只进行必要元数据检查，不读取 Excel/TXT 正文，不上传凭据。
3. **运行时实际访问**：需要额外文件访问事件捕获能力，工作量及安全/权限影响较高；**不应当作基础版已支持的功能**。如以后考虑，应单独评估并取得用户允许。

**“直接打开”** 是指在 C2 运行 PerchFlow 的那台本地 Windows 上，用户点击特定文件后由受限的本地组件调用系统默认程序（例如 Excel/记事本）打开对应文件或所在文件夹；不能只放浏览器 `file://` 链接并假定可用。应限制为用户显式确认的路径/文件，阻断未授权任意路径、远程访问、目录穿越及危险可执行文件类型，且禁止自动打开；若文件在其他电脑上，不能假定当前 C2 有其路径。

建议详情页设置「关联文件」区，按“代码发现的候选”“已确认关联”“执行产出”分层展示，并提供打开文件、打开所在文件夹、复制路径等操作。**本条“复制路径”只是可选草案，是否保留应另行确认，不应覆盖用户已决定不需要的其他产品功能。**

### 当前源代码检查：已有能力与缺口

- 现有 [`ProjectDef`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/config/model.rs#L82-L97) 已支持一个项目关联 `tasks: Vec<String>`；每任务有 [`TaskDef`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/config/model.rs#L62-L80) 的命令、参数、目录和 Cron，所以**多任务的基本后端关联模型已有基础**。
- [项目页](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/projects.html) 与 [项目 JS](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/assets/projects.js) 目前在项目右侧卡片内列出关联任务，没有“每个 Python 入口对应独立进程详情页”的结构。
- [`TaskRuntime`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/core/state.rs#L47-L54) 有状态、开始/结束时间和 run_id，**没有 PID、CPU、内存等实时指标字段**；相反 `ProcRuntime` 有 PID 和资源指标。不能在 UI 中直接把项目任务的 PID 当成现成能力。
- [调度器](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/core/scheduler.rs) 已启动实际子进程、支持日志和任务取消，但任务详情需要新增“子进程 PID / 进程指标”的安全采集和生命周期映射；要避免误显示/误操作重复 PID。
- [项目文件接口](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/web/api_project.rs#L44-L77) 只查看**事先填写**的输入、输出路径的存在/大小/修改时间，不会分析 `.py` 依赖关系，也不会读取 Excel/TXT 内容；当前没有经过安全约束的“用本地关联程序打开文件”功能。
- [进程详情](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/frontend/process-detail.html) 已有 PID、CPU、内存、运行时长和实时日志的 UI，可作为新 Python 任务详情页**视觉与交互参考**，但不能直接假定两类后端 API 完全兼容。

### 建议验收标准（未来，仅备忘）

1. `Photos_AuUp` 项目可同时登记 `Photo_AutoPublisher.py` 和 `sync_photos_to_excel.py`，二者在项目中分别占一整行，任务状态、PID、日志、历史不串台。
2. 从任一脚本行进入各自详情；运行中展示真实 PID、运行时间、可用资源指标；结束后显示退出码和关联日志。
3. 可发现常见静态外部路径（比如 Excel/TXT），不确定路径标注“待确认”，不因识别不到就报错；用户能补充手工关联。
4. 用户可点击明确受信任的 TXT/Excel 文件，在 C2 的默认程序中打开；不能误打开不相关本地文件或执行程序。
5. 多脚本的增删关联只影响 PerchFlow 登记，不删除原 `.py`、Excel、TXT 文件，也不自行运行或修改发布任务。
6. 真实脚本默认 **Cron 禁用、没有隐式执行**；保留旧运行历史、项目配置、C2 现有业务数据。
7. 与原进程/定时任务 UI 视觉一致，宽屏和窄屏均可用。

### 开发前仍需确认（不阻碍现在的记录）

- 扫描 `.py` 是“仅列出候选后手动选”，还是连带自动关联已有任务；都不得自动运行。
- 各 Python 任务详情是否基于现有进程详情建立新页面；首批交付哪些指标。
- “直接打开”对于外部文件是否仅限同机 C2、是否需要文件夹打开和文件白名单。
- 文件扫描深度及支持哪些动态配置形式，第一阶段应避免大范围递归或扫描凭据正文。

**执行边界**：仅需求记录；未经用户明确授权，不做项目 UI 重构、不扫描 C2 磁盘、不修改/执行 `Photo_AutoPublisher.py` 或 `sync_photos_to_excel.py`，不重新编译和部署。

---

## PF-004 — 每个 Python 脚本日志只保留最近三次；每次启动重新从 0 开始

- **反馈日期**：2026-10-21。
- **类别**：日志生命周期与展示行为的新增需求，**尚未实施**。
- **关联需求**：PF-003（项目内每个 Python 脚本有独立运行详情与实时日志），PF-001（运行状态一致性）。
- **状态**：仅登记与源码检查；未经用户许可不得修改代码或清理现有日志。

### 用户的要求

> 针对项目中的每个具体脚本，日志只保留近三次。每次开始执行这个脚本，日志从 0 开始，而不是接在上一次运行的日志后面。

**理解为每个脚本/任务独立管理三份最近执行日志**，而不是整个项目总共只保留三份，也不是共用一个持续追加的日志文件。

示例：`Photos_AuUp` 的 `Photo_AutoPublisher.py` 与 `sync_photos_to_excel.py` 各有自己的近三次记录，不串日志。

| Photo_AutoPublisher.py 执行 | 当前可保留的该脚本日志 | 被清理的历史日志 |
| --- | --- | --- |
| 第 1 次 | Run 001 | 无 |
| 第 2 次 | Run 001、002 | 无 |
| 第 3 次 | Run 001、002、003 | 无 |
| 第 4 次 | Run 002、003、004 | Run 001 |

`sync_photos_to_excel.py` 的三次配额不受上述执行影响。

### 预期细节

1. **每个 run_id 一份独立日志**：新运行开始时先显示 0 行/空白日志，首条输出出现后才从 1 行向上计数；自动实时跟踪当前 run_id，不自动拼接之前运行的日志。
2. **只保留最近三份 PerchFlow 管理的脚本运行日志**：第四次开始并创建新日志后清理该脚本最旧一份，以此类推。运行中、正常成功、失败、取消、超时的实际执行应按相同规则计算；未真正启动脚本而被输入预检阻断的记录应明确区分，避免误占日志名额。
3. **用户可手动查看保留范围内的旧日志**，例如第 2、3 次；默认打开当前/最近一次。
4. **保留策略只作用于 PerchFlow 在测试包自身 `task-logs` 中生成的该任务的日志**；不删除 Python 脚本自行写入项目目录的 TXT、Excel、CSV 或其他程序日志，也不触碰 C2 现有业务文件。
5. **执行历史摘要与日志正文是两类数据**：建议历史状态、耗时和退出码继续保留（如旧记录已经没有日志则显示「日志已按三次保留规则清理」）；是否连同历史摘要也只保留三次，用户尚未要求。
6. **清理需要可恢复性和保护措施**：严格按登记的任务标识及运行 ID 定位 PerchFlow 自己生成的日志文件，只删除超出保留数的已结束运行日志；服务重启仍执行相同规则，清理失败时不影响新脚本执行；不跨任务或跨项目清理。
7. 如果同一个 `.py` 路径被注册为多个不同名称的任务，需要明确“按任务配置”还是“按文件路径”聚合；默认建议**按任务配置独立**，因为参数/解释器可能不同，此规则可在实施前确认。

### 当前实现事实（源代码验证）

- [`scheduler.rs`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/core/scheduler.rs#L246-L255) 已根据安全任务名 + `run_id` 使用 `File::create(... .log)` 创建独立日志文件，**单次日志文件原本就是新建、从空白开始**，不会把不同 `run_id` 的正文写入同一文件。需要统一的是**详情页面的切换/清空展示、日志保留数量及自动清理**。
- [`api_task.rs`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/web/api_task.rs#L217-L242) 可按任务 + `run_id` 读取对应日志的最后 128 KiB；此处目前没有“三次保留”判断或针对已清理日志的友好提示。
- [`scheduler.rs`](https://github.com/buyuuz/PerchFlow/blob/feature/project-hub-live-runs/src/core/scheduler.rs#L228-L244) 的 `history.jsonl` 是**统一持续追加的执行历史索引**，并非上述三次日志正文；不应在未经确认时直接清空它。
- 日志保留 **3 次**、删除第 4 次之前的旧 `.log` 文件、展示从 0 行计数等仍属于**待做需求**，目前不能说已实现。

### 未来验收用例（未执行）

1. A.py 第 1～4 次运行仅剩 A 的 2/3/4 号日志，启动第 4 次的页面日志初始为 0 行，再逐步增长；B.py 仍保留自己的全部（至多三份）。
2. 失败/手动停止/超时均各自保留独立日志；退出后最新运行仍能查看；被清理的旧运行不再返回日志正文。
3. 重启 PerchFlow 后“三次”保留行为正确；不能误删脚本数据文件、其他任务日志或用户自己管理的日志。
4. UI 明确区分“本次没有输出”、“日志仍在生成”、“旧日志已清理”，不把空日志伪装为错误。
5. 不隐式触发发布、不重试已排期的实际业务任务。

**执行边界**：当前仅记录、排查、整理，不修改程序、不重新构建、不删除任何现存日志或业务文件。等待用户明确说“可以开始修复”后再实施。

---
后续问题按 PF-005、PF-006… 继续追加；只有用户明确授权后才统一修复。
