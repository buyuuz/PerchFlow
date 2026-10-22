/* Project registry: metadata only. Never request file contents or expose secrets. */
const projectTranslationsEn = {
  "还没有项目，点击「新建项目」开始。": "No projects yet. Choose “New project” to get started.",
  "查看 ": "View ",
  "移除脚本": "Remove script",
  "确认移除": "Remove",
  "请先停止脚本": "Stop the script first",
  "仅移除管理配置和定时计划，不删除 Python 文件": "Remove the managed configuration and schedule; the Python file remains unchanged",
  "正在检查输入依赖…": "Checking required inputs…",
  "输入预检：未开启（旧任务行为保持不变）": "Input preflight: off (legacy behavior unchanged)",
  "输入预检：已就绪 · 已检查 ": "Input preflight: ready · checked ",
  " 项": " items",
  "输入预检：未就绪 · ": "Input preflight: not ready · ",
  "不可用 ": "unavailable ",
  "输入预检状态暂不可用": "Input readiness is temporarily unavailable",
  "暂无关联任务。可先在「定时任务」创建，再编辑项目进行关联。": "No linked tasks yet. Create a task in Tasks and link it here.",
  "状态：": "Status: ",
  "已启用": "enabled",
  "已暂停": "paused",
  "任务未找到": "Task not found",
  "未设置调度": "No schedule",
  "运行一次": "Run once",
  "停止": "Stop",
  "暂停定时": "Pause Cron",
  "启用定时": "Enable Cron",
  "执行详情": "Run details",
  "任务配置 ↗": "Task settings ↗",
  "尚未配置输入或输出文件。": "No input or output paths configured.",
  "输入": "Input",
  "输出": "Output",
  "自动扫描目录": "Tracked directory",
  "文件": "File",
  "大小 ": "Size ",
  " · 修改 ": " · Modified ",
  "存在": "Available",
  "未找到": "Missing",
  "空闲": "Idle",
  "运行中": "Running",
  "成功": "Success",
  "失败": "Failed",
  "已取消": "Cancelled",
  "预检阻止": "Preflight blocked",
  "超时": "Timed out",
  "意外中断": "Interrupted",
  "未知": "Unknown",
  "现在执行「": "Run “",
  "」？这将启动配置的真实脚本，即使定时调度已暂停。": "” now? This launches the configured real script even if Cron is paused.",
  "暂停": "Pause",
  "启用": "Enable",
  "」的 Cron 定时？": "” Cron schedule?",
  " 正在运行的任务不会被终止。": " Active tasks will not be stopped.",
  " 启用后将根据 Cron 自动执行脚本。": " Enabling this will automatically run the script on its Cron schedule.",
  "停止「": "Stop “",
  "」当前正在运行的任务？": "” currently running task?",
  "编辑项目": "Edit project",
  "新建项目": "New project",
  "尚无任务。请先在「定时任务」页面添加 Python 启动任务。": "No tasks yet. Add a Python task in Tasks first.",
  "（已属于 ": " (already linked to ",
  "请填写项目名称和完整工作目录。": "Enter a project name and an absolute directory.",
  "最多配置 8 个输出目录。": "Configure no more than 8 output directories.",
  "删除项目「": "Delete project “",
  "」的管理关联？不会删除关联任务和任何文件。": "” links? Tasks and files will not be deleted.",
  " · 最近执行": " · Recent runs",
  "正在加载执行记录…": "Loading run history…",
  "暂无运行记录。": "No runs yet.",
  " · 执行摘要": " · Summary",
  "运行 ID：": "Run ID: ",
  " · 开始：": " · Started: ",
  " · 结束：": " · Finished: ",
  "执行状态：": "Status: ",
  " · 耗时：": " · Duration: ",
  " 秒": " sec",
  " · 退出码：": " · Exit code: ",
  "执行异常：": "Run error: ",
  "输入预检：通过（": "Input preflight: passed (",
  " 项）": " items)",
  "输入预检：阻止执行 · 不可用文件：": "Input preflight: blocked · unavailable files: ",
  "本次产出变化：新增 ": "Output changes: created ",
  " · 删除 ": " · removed ",
  "扫描不完整，结果仅供参考。": "Partial scan; results may be incomplete.",
  "按需查看该次日志": "View this run’s log",
  "正在读取日志…": "Loading log…",
  "提示：脚本日志可能包含其自行打印的敏感内容。": "Warning: the script may have printed sensitive information in its log.",
  "（空日志）": "(Empty log)",
  "只显示最后 128 KiB 日志。": "Showing the last 128 KiB of the log.",
  "该次执行没有可用日志。": "No log available for this run.",
  "正在读取文件变化…": "Loading file changes…",
  "检查了 ": "Checked ",
  " 条文件/目录元数据（含自动发现）；不包含文件正文。": " file/directory metadata entries (including discovery); no file contents.",
  "扫描未完整完成（文件数量/目录深度或访问权限限制），此结果可能不包含所有变化。": "Scan incomplete (file limits, directory depth or permissions); some changes may be missing.",
  "没有检测到已登记文件的变化。": "No changes to tracked files detected.",
  "输出目录": "Output folder",
  "自动发现": "Discovered",
  "新增": "Created",
  "修改": "Modified",
  "删除": "Removed",
  "该次执行没有可用的文件变化记录（例如旧记录或未关联项目）。": "No file-change record is available (for example, for legacy runs).",
  "运行超时时间必须是非负整数。": "Timeout must be a nonnegative integer.",
  "安全检查失败：任务不应自动启用": "Safety check failed: newly created task should remain disabled.",
  "执行详情自动刷新中（约 2 秒）": "Run details auto-refreshing (about every 2 sec)",
  "当前任务正在运行，结束后显示文件变化。": "Task running; file changes will appear after completion.",
  "实时日志更新中（约 2 秒）": "Live log updates (about every 2 sec)",
  "等待脚本输出…": "Waiting for script output…",
  "历史记录刷新失败，将自动重试。": "Run history refresh failed; retrying automatically.",
  "任务已经结束，状态已更新。": "Task already finished; status updated.",
  "尚未执行": "Not run",
  "最近已执行": "Recently executed",
  "手动执行": "Manual run",
  "定时已暂停": "Cron paused",
  "已启用定时": "Scheduled",
  "运行": "Run",
  "详情": "Details",
  "暂无脚本，点击「添加脚本」开始。": "No scripts yet. Choose Add script to get started.",
  "手动运行不受 Cron 开关影响；如有已暂停的定时配置，请在定时任务页面管理。": "Manual runs are available. Manage paused Cron settings on the Tasks page.",
  "在定时任务页面查看高级设置": "Advanced settings in Tasks",  "详情 ↗": "Details ↗"
};
function pfL(zh) {
  return window.getLang && window.getLang() === 'en'
    ? (projectTranslationsEn[zh] || zh) : zh;
}
function localizeProjectStatic() {
  document.querySelectorAll('[data-pf-en]').forEach(node => {
    if (!node.hasAttribute('data-pf-zh')) node.setAttribute('data-pf-zh', node.textContent);
    node.textContent = (window.getLang && window.getLang() === 'en')
      ? node.getAttribute('data-pf-en') : node.getAttribute('data-pf-zh');
  });
  document.title = (window.getLang && window.getLang() === 'en')
    ? 'PerchFlow · Projects' : 'PerchFlow · 项目管理';
}


'use strict';

let projects = [];
let taskViews = [];
let currentProjectName = null;
let editedProjectName = null;
let runMonitorGeneration = 0;
const busyTasks = new Set();
let lastHistoryTask = null;
let historyRequestId = 0;
let fileRequestId = 0;
let selectedRunId = null;
let taskStatusPolling = false;
let runDetailsTimer = null;
let runDetailsLoading = false;
let runDetailsProject = null;
let runDetailsTask = null;
let autoFollowNewestRun = true;
let runHistorySignature = '';
let runSummarySignature = '';
let runLogOptIn = false;
let runLogFinalRead = false;
let runLogLoading = false;
let currentRunStatus = null;

function el(tag, className, value) {
  const n = document.createElement(tag);
  if (className) n.className = className;
  if (value != null) n.textContent = String(value);
  return n;
}

function byId(name) { return document.getElementById(name); }
function displayError(err) {
  const box = byId('project-notice');
  box.textContent = String(err.message || err);
  box.hidden = false;
}
function clearError() { byId('project-notice').hidden = true; }

async function requestJson(url, method, body) {
  const options = method ? {method:method} : {};
  if (body !== undefined) options.body = JSON.stringify(body);
  const response = await api(url, options);
  const data = await response.json();
  if (!response.ok || data.ok === false) throw new Error(data.error || 'Request failed');
  return data;
}

async function loadProjects(selectName) {
  clearError();
  try {
    const responses = await Promise.all([
      requestJson('/api/projects'),
      requestJson('/api/tasks'),
    ]);
    projects = Array.isArray(responses[0].projects) ? responses[0].projects : [];
    taskViews = Array.isArray(responses[1].tasks) ? responses[1].tasks : [];
    if (selectName !== undefined) currentProjectName = selectName;
    else if (!currentProjectName) currentProjectName = new URLSearchParams(window.location.search).get('project');
    if (!projects.some(p => p.name === currentProjectName)) {
      currentProjectName = projects.length ? projects[0].name : null;
    }
    const selectedProject = projects.find(p => p.name === currentProjectName);
    if (!selectedProject || !selectedProject.tasks.includes(lastHistoryTask)) lastHistoryTask = null;
    renderProjectList();
    await renderProjectDetail();
  } catch (error) { displayError(error); }
}

function renderProjectList() {
  const target = byId('project-list');
  target.replaceChildren();
  byId('project-count').textContent = projects.length;
  if (!projects.length) {
    target.append(el('p', 'pf-project-muted', pfL('还没有项目，点击「新建项目」开始。')));
    return;
  }
  projects.forEach(project => {
    const button = el('button', 'pf-project-entry');
    button.type = 'button';
    button.setAttribute('aria-selected', String(project.name === currentProjectName));
    button.setAttribute('aria-label', pfL('查看 ') + project.name);
    const left = el('span');
    left.append(el('strong', '', project.name));
    left.append(el('small', '', project.root));
    button.append(left, el('span', 'material-symbols-outlined', 'folder'));
    button.addEventListener('click', async () => {
      runMonitorGeneration += 1;
      stopRunDetailsPolling();
      historyRequestId += 1;
      fileRequestId += 1;
      selectedRunId = null;
      currentProjectName = project.name;
      renderProjectList();
      try { await renderProjectDetail(); } catch (error) { displayError(error); }
    });
    target.append(button);
  });
}

function formattedBytes(bytes) {
  if (bytes == null || !Number.isFinite(Number(bytes))) return '—';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KiB';
  return (bytes / 1048576).toFixed(1) + ' MiB';
}

function taskScriptFileName(task) {
  const path = task?.args?.find(arg => String(arg).toLowerCase().endsWith('.py')) || '';
  return path ? String(path).replace(/\\/g, '/').split('/').pop() : '';
}
function taskDetailHref(name) {
  return '/task-detail?project=' + encodeURIComponent(currentProjectName) + '&task=' + encodeURIComponent(name);
}
function taskLastResult(task) {
  if (!task || !task.last_run) return {label: pfL('尚未执行'), tone:'none'};
  const raw = task.status === 'running' ? null : task.status;
  const status = ['success','failed','cancelled','blocked','timeout','interrupted'].includes(raw)
    ? raw : (task.last_exit_code === 0 ? 'success' : task.last_exit_code != null ? 'failed' : null);
  return status
    ? {label:statusText(status),tone: status === 'success' ? 'success' : 'error'}
    : {label:pfL('最近已执行'),tone:'none'};
}
function updateProjectTaskRow(row, task) {
  const name = row.dataset.taskName;
  const running = task?.status === 'running';
  const status = row.querySelector('[data-task-state]');
  if (status) {
    status.textContent = running ? pfL('运行中') : pfL('空闲');
    status.classList.toggle('running',running);
  }
  const result = taskLastResult(task);
  const outcome = row.querySelector('[data-task-result]');
  if (outcome) {
    outcome.textContent = result.label;
    outcome.dataset.tone = result.tone;
  }
  const date = row.querySelector('[data-task-date]');
  if (date) date.textContent = task?.last_run ? new Date(task.last_run).toLocaleString() : '—';
  const schedule = row.querySelector('[data-task-mode]');
  if (schedule) {
    schedule.textContent = task?.schedule ? (task.enabled ? pfL('已启用定时') : pfL('定时已暂停')) : pfL('手动执行');
    schedule.dataset.scheduled = String(Boolean(task?.schedule));
    schedule.title = task?.schedule ? task.schedule :
      pfL('手动运行不受 Cron 开关影响；如有已暂停的定时配置，请在定时任务页面管理。');
  }
  const run = row.querySelector('[data-task-action="run"]');
  const stop = row.querySelector('[data-task-action="stop"]');
  if (run) {run.disabled = !task || running || busyTasks.has(name); run.hidden = running;}
  if (stop) {stop.disabled = !task || !running || busyTasks.has(name); stop.hidden = !running;}
  const remove = row.querySelector('.pf-project-delete-script');
  if(remove)remove.disabled = !task || running || busyTasks.has(name);
}
async function renderProjectDetail() {
  const placeholder = byId('project-empty'), target = byId('project-detail');
  placeholder.hidden = Boolean(currentProjectName);
  target.hidden = !currentProjectName;
  if (!currentProjectName) return;
  const selected = currentProjectName;
  const data = await requestJson('/api/projects/' + encodeURIComponent(selected));
  if (currentProjectName !== selected) return;
  const project = data.project;
  byId('project-current-name').textContent = project.name;
  byId('project-root-preview').textContent = project.root;
  byId('project-actions-menu').hidden = true;
  byId('project-more').setAttribute('aria-expanded','false');
  stopRunDetailsPolling();
  byId('project-runs').hidden = true;
  const jobs = byId('project-jobs');
  jobs.replaceChildren();
  if (!project.tasks.length) {
    jobs.append(el('p','pf-project-empty-scripts',pfL('暂无脚本，点击「添加脚本」开始。')));
    return;
  }
  project.tasks.forEach(name => {
    const task = taskViews.find(t => t.name === name);
    const row = el('div','pf-project-task-item');
    row.role = 'row';
    row.dataset.taskName = name;
    const info = el('div','pf-project-task-main');
    info.role = 'cell';
    const scriptIcon = el('span','material-symbols-outlined pf-project-script-icon','description');
    scriptIcon.setAttribute('aria-hidden','true');
    const anchor = el('a','pf-project-task-link');
    anchor.href = taskDetailHref(name);
    anchor.title = name;
    anchor.append(el('span','pf-project-task-title',name));
    const scriptName = taskScriptFileName(task);
    if (scriptName) anchor.append(el('small','pf-project-task-filename',scriptName));
    info.append(scriptIcon,anchor);
    const state = el('div','pf-project-cell pf-project-status-cell');
    state.role = 'cell';
    state.append(el('span','pf-project-status-dot'));
    const label = el('span','pf-project-status');
    label.dataset.taskState = 'true';
    state.append(label);
    const last = el('div','pf-project-cell pf-project-last-result');
    last.role = 'cell';
    const result = el('span','pf-project-result');
    result.dataset.taskResult = 'true';
    const when = el('small','pf-project-result-date');
    when.dataset.taskDate = 'true';
    last.append(result,when);
    const mode = el('div','pf-project-cell pf-project-mode-cell');
    mode.role = 'cell';
    const modeValue = el('span','pf-project-mode');
    modeValue.dataset.taskMode = 'true';
    mode.append(el('span','material-symbols-outlined','schedule'),modeValue);
    const actions = el('div','pf-project-action-row pf-project-task-actions');
    actions.role = 'cell';
    const run = actionButton(pfL('运行'),'pf-project-primary',() => operateProjectTask(name,'run'));
    run.dataset.taskAction = 'run';
    const stop = actionButton(pfL('停止'),'pf-project-danger',() => operateProjectTask(name,'stop'));
    stop.dataset.taskAction = 'stop';
    const detail = el('a','pf-project-secondary pf-project-detail-link',pfL('详情'));
    detail.href = taskDetailHref(name);
    const remove = actionButton(pfL('移除脚本'),'pf-project-danger pf-project-delete-script',
      () => {void deleteProjectScript(name);});
    remove.disabled = !task || task.status === 'running';
    remove.title = task?.status === 'running' ? pfL('请先停止脚本') : pfL('仅移除管理配置和定时计划，不删除 Python 文件');
    actions.append(run,stop,detail,remove);
    row.append(info,state,last,mode,actions);
    row.addEventListener('click',event=>{
      if(!event.target.closest('button,a,input')) window.location.assign(taskDetailHref(name));
    });
    jobs.append(row);
    updateProjectTaskRow(row,task);
  });
}
const TASK_STATUSES = {
  idle:'空闲', running:'运行中', success:'成功', failed:'失败',
  cancelled:'已取消', blocked:'预检阻止', timeout:'超时', interrupted:'意外中断'
};
function statusText(status) { return pfL(TASK_STATUSES[status] || status || '未知'); }
function actionButton(label, style, onClick) {
  const button = el('button', style, label);
  button.type = 'button';
  button.addEventListener('click', onClick);
  return button;
}

// Independent of a manual run. Keep task cards and Stop buttons in sync for
// Python jobs that run longer than 90 seconds or start via scheduled Cron.
// Do not poll logs or project file bodies from this lightweight refresh.
async function syncVisibleProjectTasks() {
  if (taskStatusPolling || document.hidden || !currentProjectName) return;
  const selected = currentProjectName;
  taskStatusPolling = true;
  try {
    const result = await requestJson('/api/tasks');
    if (selected !== currentProjectName) return;
    taskViews = Array.isArray(result.tasks) ? result.tasks : [];
    document.querySelectorAll('.pf-project-task-item').forEach(row => {
      const task = taskViews.find(item => item.name === row.dataset.taskName);
      updateProjectTaskRow(row,task);
    });
  } catch (_) { /* Use manual refresh for error details. */ }
  finally { taskStatusPolling = false; }
}
async function refreshProjectState() {
  const selected = currentProjectName;
  if (selected) await loadProjects(selected);
}
async function operateProjectTask(name, action) {
  const projectName = currentProjectName;
  const project = projects.find(p => p.name === projectName);
  const task = taskViews.find(t => t.name === name);
  if (!project?.tasks.includes(name) || !task || busyTasks.has(name)) return;
  const url = '/api/tasks/' + encodeURIComponent(name);
  if (action === 'schedule' && !await PerchDialog.confirm(
    (task.enabled ? '暂停' : '启用') + '「' + name + '」的 Cron 定时？',
    {title:'定时计划状态'}
  )) return;
  if (action === 'stop' && !await PerchDialog.confirm('停止「' + name + '」当前运行的任务？',
    {title:'停止脚本',danger:true})) return;
  busyTasks.add(name);
  try {
    let result;
    if (action === 'schedule') {
      result = await requestJson('/api/projects/' + encodeURIComponent(projectName) +
        '/tasks/' + encodeURIComponent(name) + '/schedule', 'PUT', {enabled: !task.enabled});
    } else {
      result = await requestJson(url + (action === 'run' ? '/run' : '/stop'), 'POST');
    }
    // Release the pending-action lock before rerendering the task buttons.
    busyTasks.delete(name);
    if (currentProjectName === projectName) {
      await refreshProjectState(action === 'schedule' ? null : name);
      if (action === 'stop' && result.already_finished) {
        byId('project-notice').textContent = pfL('任务已经结束，状态已更新。');
        byId('project-notice').hidden = false;
      }
      if (action === 'run' && result.run_id) {
        const generation = ++runMonitorGeneration;
        void monitorTaskRun(projectName, name, result.run_id, generation);
      }
    }
  } catch (error) { displayError(error); }
  finally {
    busyTasks.delete(name);
    if (action === 'stop') void syncVisibleProjectTasks();
  }
}

// Short-lived status polling after a user-initiated run, not a background service.
// A manual refresh remains available for long-running jobs.
async function monitorTaskRun(projectName, name, runId, generation) {
  for (let attempt = 0; attempt < 90; attempt++) {
    await new Promise(resolve => setTimeout(resolve, 1000));
    if (generation !== runMonitorGeneration || currentProjectName !== projectName) return;
    try {
      const task = await requestJson('/api/tasks/' + encodeURIComponent(name));
      const row = Array.from(document.querySelectorAll('.pf-project-task-item'))
        .find(node => node.dataset.taskName === name);
      if (row) updateProjectTaskRow(row,task);
      if (task.run_id === runId && task.status !== 'running') {
        await refreshProjectState();
        return;
      }
    } catch (_) { return; }
  }
}
function lines(id) {
  return byId(id).value.split(/\r?\n/).map(x => x.trim()).filter(Boolean);
}

function openProjectForm(projectName) {
  editedProjectName = projectName || null;
  const project = projects.find(p => p.name === editedProjectName);
  const saveButton = byId('project-save');
  saveButton.setAttribute('data-pf-zh', project ? '保存修改' : '创建项目');
  saveButton.setAttribute('data-pf-en', project ? 'Save changes' : 'Create project');
  saveButton.textContent = window.getLang && window.getLang() === 'en' ? saveButton.getAttribute('data-pf-en') : saveButton.getAttribute('data-pf-zh');
  byId('project-form-title').textContent = project ? pfL('编辑项目') : pfL('新建项目');
  byId('project-name').value = project ? project.name : '';
  byId('project-root').value = project ? project.root : '';
  byId('project-inputs').value = project ? project.inputs.join('\n') : '';
  byId('project-outputs').value = project ? project.outputs.join('\n') : '';
  byId('project-output-dirs').value = project ? (project.output_dirs || []).join('\n') : '';
  byId('project-preflight-inputs').checked = Boolean(project && project.preflight_inputs);
  byId('project-form-error').hidden = true;
  const picker = byId('project-task-options');
  picker.replaceChildren();
  if (!taskViews.length) picker.append(el('p','pf-project-muted',pfL('尚无任务。请先在「定时任务」页面添加 Python 启动任务。')));
  taskViews.forEach(task => {
    const assigned = projects.find(p => p.name !== editedProjectName && p.tasks.includes(task.name));
    const label = el('label');
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.value = task.name;
    input.checked = Boolean(project && project.tasks.includes(task.name));
    input.disabled = Boolean(assigned);
    label.append(input, document.createTextNode(task.name + (assigned ? pfL('（已属于 ') + assigned.name + '）' : '')));
    picker.append(label);
  });
  byId('project-modal').hidden = false;
  byId('project-form').scrollTop = 0;
  byId('project-name').focus({preventScroll:true});
}

function closeProjectForm() {
  byId('project-modal').hidden = true;
}

async function saveProject(event) {
  event.preventDefault();
  const errorBox = byId('project-form-error');
  errorBox.hidden = true;
  const name = byId('project-name').value.trim();
  const root = byId('project-root').value.trim();
  const tasks = Array.from(byId('project-task-options').querySelectorAll('input:checked')).map(n => n.value);
  const payload = {
    name:name, root:root, tasks:tasks,
    inputs:lines('project-inputs'), outputs:lines('project-outputs'),
    output_dirs:lines('project-output-dirs'),
    preflight_inputs:byId('project-preflight-inputs').checked
  };
  if (!name || !root) {
    errorBox.textContent = pfL('请填写项目名称和完整工作目录。');
    errorBox.hidden = false;
    return;
  }
  if (payload.output_dirs.length > 8) {
    errorBox.textContent = pfL('最多配置 8 个输出目录。');
    errorBox.hidden = false;
    return;
  }
  const button = byId('project-save');
  button.disabled = true;
  try {
    const oldName = editedProjectName;
    await requestJson(oldName ? '/api/projects/' + encodeURIComponent(oldName) : '/api/projects',
                      oldName ? 'PUT' : 'POST', payload);
    closeProjectForm();
    await loadProjects(name);
  } catch (error) {
    errorBox.textContent = error.message || String(error);
    errorBox.hidden = false;
  } finally { button.disabled = false; }
}

async function deleteProjectScript(name) {
  const selected=currentProjectName;
  const task=taskViews.find(item=>item.name===name);
  if(!selected || !task || busyTasks.has(name) || task.status==='running')return;
  const yes=await PerchDialog.confirm(
    '仅移除「'+name+'」在 PerchFlow 中的任务定义、项目关联及 Cron 计划。不会删除磁盘上的 .py、Excel、TXT 等文件。',
    {title:pfL('移除脚本'),ok:pfL('确认移除'),danger:true});
  if(!yes)return;
  busyTasks.add(name);
  try{
    await requestJson('/api/projects/'+encodeURIComponent(selected)+
      '/tasks/'+encodeURIComponent(name),'DELETE');
    if(currentProjectName===selected)await loadProjects(selected);
  }catch(error){displayError(error);}
  finally{busyTasks.delete(name);}
}

async function deleteProject() {
  if (!currentProjectName) return;
  const name = currentProjectName;
  if (!await PerchDialog.confirm(pfL('删除项目「') + name + pfL('」的管理关联？不会删除关联任务和任何文件。'),{title:'删除项目',danger:true})) return;
  try {
    await requestJson('/api/projects/' + encodeURIComponent(name), 'DELETE');
    await loadProjects(null);
  } catch (error) { displayError(error); }
}

// Poll only while the user has the run-details panel open. Historical metadata
// refreshes automatically; sensitive stdout/stderr is NEVER fetched until the
// user explicitly clicks "View this run's log".
function stopRunDetailsPolling() {
  if (runDetailsTimer !== null) {
    clearInterval(runDetailsTimer);
    runDetailsTimer = null;
  }
  runDetailsProject = null;
  runDetailsTask = null;
  runDetailsLoading = false;
  runLogOptIn = false;
  runLogFinalRead = false;
  runHistorySignature = '';
  runSummarySignature = '';
  historyRequestId += 1;
  fileRequestId += 1;
}

async function showProjectRuns(name) {
  if (!projects.some(p => p.name === currentProjectName && p.tasks.includes(name))) return;
  stopRunDetailsPolling();
  runDetailsProject = currentProjectName;
  runDetailsTask = name;
  autoFollowNewestRun = true;
  selectedRunId = null;
  byId('project-runs').hidden = false;
  byId('project-runs-title').textContent = name + pfL(' · 最近执行');
  lastHistoryTask = name;
  byId('project-runs-list').replaceChildren(el('p', 'pf-project-muted', pfL('正在加载执行记录…')));
  byId('project-run-changes').replaceChildren();
  byId('project-run-summary').replaceChildren();
  byId('project-run-log').replaceChildren();
  byId('project-run-log').hidden = true;
  await refreshRunDetails();
  if (runDetailsTask === name && runDetailsProject === currentProjectName && !byId('project-runs').hidden) {
    runDetailsTimer = setInterval(() => {
      if (!document.hidden) void refreshRunDetails();
    }, 2000);
  }
}

async function refreshRunDetails() {
  if (runDetailsLoading || byId('project-runs').hidden || !runDetailsTask) return;
  const project = runDetailsProject;
  const name = runDetailsTask;
  const requestId = historyRequestId;
  runDetailsLoading = true;
  try {
    const response = await requestJson('/api/tasks/' + encodeURIComponent(name) + '/history');
    if (requestId !== historyRequestId || project !== currentProjectName ||
        name !== runDetailsTask || byId('project-runs').hidden) return;
    const runs = (response.runs || []).slice(0, 12);
    const signature = JSON.stringify(runs.map(r => [r.run_id, r.status, r.finished_at, r.duration_ms, r.exit_code, r.output_summary]));
    if (signature !== runHistorySignature) {
      runHistorySignature = signature;
      const list = byId('project-runs-list');
      list.replaceChildren();
      if (!runs.length) list.append(el('p', 'pf-project-muted', pfL('暂无运行记录。')));
      for (const run of runs) {
        const button = el('button', 'pf-project-run-item',
          (run.started_at || '—') + ' · ' + statusText(run.status) + pfL(' · 执行摘要'));
        button.type = 'button';
        button.addEventListener('click', () => {
          autoFollowNewestRun = false;
          void showRunSummary(name, run);
        });
        list.append(button);
      }
    }
    const desired = (autoFollowNewestRun || !selectedRunId)
      ? runs[0] : runs.find(r => r.run_id === selectedRunId);
    if (desired) await showRunSummary(name, desired);
    if (runLogOptIn && selectedRunId && !runLogFinalRead) {
      await refreshLiveLog(name, selectedRunId, project);
    }
  } catch (error) {
    if (requestId === historyRequestId && project === currentProjectName &&
        !byId('project-runs').hidden) {
      const list = byId('project-runs-list');
      if (!runHistorySignature) {
        list.replaceChildren(el('p', 'pf-project-form-error', pfL('历史记录刷新失败，将自动重试。')));
      }
    }
  } finally {
    runDetailsLoading = false;
  }
}

async function showRunSummary(name, run) {
  if (name !== runDetailsTask || !run?.run_id ||
      !projects.some(p => p.name === currentProjectName && p.tasks.includes(name))) return;
  const project = currentProjectName;
  const isNewRun = selectedRunId !== run.run_id;
  if (isNewRun) {
    selectedRunId = run.run_id;
    fileRequestId += 1;
    runSummarySignature = '';
    runLogOptIn = false;
    runLogFinalRead = false;
    const log = byId('project-run-log');
    log.hidden = true;
    log.replaceChildren();
  }
  currentRunStatus = run.status;
  const signature = JSON.stringify([run.run_id, run.status, run.finished_at, run.duration_ms,
    run.exit_code, run.preflight, run.output_summary, run.error]);
  if (runSummarySignature === signature) return;
  runSummarySignature = signature;
  const requestId = fileRequestId;
  const target = byId('project-run-summary');
  target.replaceChildren();
  const status = statusText(run.status);
  target.append(el('p', 'pf-project-muted', pfL('运行 ID：') + run.run_id +
    pfL(' · 开始：') + (run.started_at || '—') + pfL(' · 结束：') + (run.finished_at || '—')));
  target.append(el('p', 'pf-project-run-metric', pfL('执行状态：') + status +
    pfL(' · 耗时：') + (run.duration_ms == null ? '—' : (run.duration_ms / 1000).toFixed(2) + pfL(' 秒')) +
    pfL(' · 退出码：') + (run.exit_code == null ? '—' : run.exit_code)));
  if (run.error) target.append(el('p', 'pf-project-form-error', pfL('执行异常：') + String(run.error)));
  if (run.preflight?.enabled) {
    const info = run.preflight;
    target.append(el('p', info.ready ? 'pf-project-muted' : 'pf-project-form-error',
      info.ready ? pfL('输入预检：通过（') + info.checked + pfL(' 项）') :
      pfL('输入预检：阻止执行 · 不可用文件：') +
      (info.unavailable || []).join('、') + (info.reason ? ' (' + info.reason + ')' : '')));
  }
  if (run.output_summary) {
    const s = run.output_summary;
    target.append(el('p', 'pf-project-run-metric',
      pfL('本次产出变化：新增 ') + s.created + pfL(' · 修改 ') + s.modified + pfL(' · 删除 ') + s.removed));
    if (s.incomplete_scan) target.append(el('p', 'pf-project-form-error', pfL('扫描不完整，结果仅供参考。')));
  }
  if (runLogOptIn) {
    target.append(el('p', 'pf-project-muted', pfL('实时日志更新中（约 2 秒）')));
  } else {
    const button = el('button', 'pf-project-secondary', pfL('按需查看该次日志'));
    button.type = 'button';
    button.addEventListener('click', () => {
      if (project !== currentProjectName || name !== runDetailsTask ||
          selectedRunId !== run.run_id) return;
      runLogOptIn = true;
      runLogFinalRead = false;
      byId('project-run-log').hidden = false;
      byId('project-run-log').replaceChildren(el('p', 'pf-project-muted', pfL('正在读取日志…')));
      void refreshLiveLog(name, run.run_id, project);
      // Replace button with passive live indicator; do not reset the log.
      button.replaceWith(el('span', 'pf-project-muted', pfL('实时日志更新中（约 2 秒）')));
    });
    target.append(button);
  }
  if (run.status === 'running') {
    byId('project-run-changes').replaceChildren(
      el('p', 'pf-project-muted', pfL('当前任务正在运行，结束后显示文件变化。')));
  } else {
    await showRunFiles(name, run.run_id, requestId, project);
  }
}

async function refreshLiveLog(name, runId, project) {
  if (!runLogOptIn || runLogFinalRead || runLogLoading ||
      runDetailsProject !== project || runDetailsTask !== name || selectedRunId !== runId) return;
  const requestId = fileRequestId;
  const terminal = currentRunStatus !== 'running';
  runLogLoading = true;
  try {
    const data = await requestJson('/api/tasks/' + encodeURIComponent(name) +
      '/log/' + encodeURIComponent(runId));
    if (requestId !== fileRequestId || project !== currentProjectName ||
        selectedRunId !== runId || !runLogOptIn || byId('project-runs').hidden) return;
    const log = byId('project-run-log');
    const existing = log.querySelector('pre');
    const content = data.content || pfL('等待脚本输出…');
    if (!existing || existing.textContent !== content ||
        Boolean(log.dataset.truncated === 'true') !== Boolean(data.truncated)) {
      const previousTop = log.scrollTop;
      const atBottom = log.scrollTop + log.clientHeight >= log.scrollHeight - 64;
      log.replaceChildren(el('p', 'pf-project-muted', pfL('提示：脚本日志可能包含其自行打印的敏感内容。')));
      if (data.truncated) log.append(el('p', 'pf-project-muted', pfL('只显示最后 128 KiB 日志。')));
      log.append(el('pre', 'pf-project-log-text', content));
      log.dataset.truncated = String(Boolean(data.truncated));
      log.scrollTop = atBottom ? log.scrollHeight : previousTop;
    }
    if (terminal) runLogFinalRead = true;
  } catch (error) {
    if (requestId === fileRequestId && project === currentProjectName &&
        selectedRunId === runId && !byId('project-runs').hidden) {
      if (!byId('project-run-log').querySelector('pre')) {
        byId('project-run-log').replaceChildren(
          el('p', 'pf-project-muted', pfL('等待脚本输出…')));
      }
    }
  } finally {
    runLogLoading = false;
  }
}

async function showRunFiles(name, runId, requestId, projectAtRequest) {
  const target = byId('project-run-changes');
  target.replaceChildren(el('p', 'pf-project-muted', pfL('正在读取文件变化…')));
  try {
    const data = await requestJson('/api/tasks/' + encodeURIComponent(name) + '/files/' + encodeURIComponent(runId));
    if (requestId !== fileRequestId || currentProjectName !== projectAtRequest ||
        selectedRunId !== runId || byId('project-runs').hidden) return;
    const record = data.record;
    target.replaceChildren();
    target.append(el('p', 'pf-project-muted', pfL('检查了 ') + record.scanned_paths +
      pfL(' 条文件/目录元数据（含自动发现）；不包含文件正文。')));
    if (record.incomplete_scan) target.append(el('p', 'pf-project-form-error',
      pfL('扫描未完整完成（文件数量/目录深度或访问权限限制），此结果可能不包含所有变化。')));
    if (!record.changes.length) target.append(el('p', 'pf-project-muted', pfL('没有检测到已登记文件的变化。')));
    for (const entry of record.changes) {
      const row = el('div', 'pf-project-file-item');
      const file = el('div', 'file-name');
      file.append(el('span', 'pf-project-file-role', entry.role === 'input' ? pfL('输入') :
        entry.role === 'output_dir' ? pfL('输出目录') :
        (entry.after?.discovered || entry.before?.discovered) ? pfL('自动发现') : pfL('输出')));
      file.append(document.createTextNode(entry.path));
      const label = {created:pfL('新增'),modified:pfL('修改'),removed:pfL('删除')}[entry.change] || entry.change;
      row.append(file, el('span', 'pf-project-status', label));
      target.append(row);
    }
  } catch (error) {
    if (requestId !== fileRequestId || currentProjectName !== projectAtRequest ||
        selectedRunId !== runId || byId('project-runs').hidden) return;
    target.replaceChildren(el('p', 'pf-project-muted', pfL('该次执行没有可用的文件变化记录（例如旧记录或未关联项目）。')));
  }
}

function openPythonForm() {
  if (!currentProjectName) return;
  byId('project-python-form').reset();

  byId('project-python-timeout').value = '3600';
  byId('project-python-error').hidden = true;
  byId('project-python-modal').hidden = false;
  byId('project-python-name').focus();
}

function closePythonForm() { byId('project-python-modal').hidden = true; }

async function savePythonTask(event) {
  event.preventDefault();
  if (!currentProjectName) return;
  const errorBox = byId('project-python-error');
  errorBox.hidden = true;
  const timeout = Number(byId('project-python-timeout').value);
  if (!Number.isSafeInteger(timeout) || timeout < 0) {
    errorBox.textContent = pfL('运行超时时间必须是非负整数。');
    errorBox.hidden = false;
    return;
  }
  const body = {
    name:byId('project-python-name').value.trim(),
    python:byId('project-python-exe').value.trim(),
    script:byId('project-python-script').value.trim(),
    args:lines('project-python-args'),
    timeout_secs:timeout
  };
  const button = byId('project-python-submit');
  button.disabled = true;
  try {
    const result = await requestJson('/api/projects/' + encodeURIComponent(currentProjectName) + '/python-task', 'POST', body);
    if (result.enabled !== false) throw new Error(pfL('安全检查失败：任务不应自动启用'));
    closePythonForm();
    await loadProjects(currentProjectName);
  } catch (error) {
    errorBox.textContent = error.message || String(error);
    errorBox.hidden = false;
  } finally { button.disabled = false; }
}

document.addEventListener('DOMContentLoaded', () => {
  localizeProjectStatic();
  byId('project-language').addEventListener('change', (event) => {
    window.setLang(event.target.value);
  });
  document.addEventListener('perchflow:language-changed', () => {
    localizeProjectStatic();
    void loadProjects(currentProjectName);
  });

  byId('project-new').addEventListener('click', () => openProjectForm(null));
  const menuButton = byId('project-more');
  const menu = byId('project-actions-menu');
  menuButton.addEventListener('click', () => {
    menu.hidden = !menu.hidden;
    menuButton.setAttribute('aria-expanded',String(!menu.hidden));
  });
  menu.addEventListener('click',() => {
    menu.hidden = true;
    menuButton.setAttribute('aria-expanded','false');
  });
  document.addEventListener('click',event => {
    if (!event.target.closest('.pf-project-menu-wrap')) {
      menu.hidden = true;
      menuButton.setAttribute('aria-expanded','false');
    }
  });
  byId('project-add-python').addEventListener('click', openPythonForm);
  byId('project-python-form').addEventListener('submit', savePythonTask);
  byId('project-python-close').addEventListener('click', closePythonForm);
  byId('project-python-cancel').addEventListener('click', closePythonForm);
  byId('project-python-modal').addEventListener('click', e => {
    if (e.target === byId('project-python-modal')) closePythonForm();
  });
  byId('project-runs-close').addEventListener('click', () => {
    byId('project-runs').hidden = true;
    stopRunDetailsPolling();
  });
  byId('project-refresh').addEventListener('click', () => {
    void refreshProjectState(lastHistoryTask).catch(displayError);
  });
  byId('project-runs-refresh').addEventListener('click', () => {
    if (lastHistoryTask) void refreshRunDetails();
  });
  byId('project-edit').addEventListener('click', () => openProjectForm(currentProjectName));
  byId('project-delete').addEventListener('click', deleteProject);
  byId('project-form').addEventListener('submit', saveProject);
  byId('project-close').addEventListener('click', closeProjectForm);
  byId('project-cancel').addEventListener('click', closeProjectForm);
  byId('project-modal').addEventListener('click', e => {
    if (e.target === byId('project-modal')) closeProjectForm();
  });
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape' && !byId('project-modal').hidden) closeProjectForm();
    if (e.key === 'Escape' && !byId('project-python-modal').hidden) closePythonForm();
  });
  void loadProjects();
  setInterval(() => { void syncVisibleProjectTasks(); }, 2500);
});
