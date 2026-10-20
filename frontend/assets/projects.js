/* Project registry: metadata only. Never request file contents or expose secrets. */
'use strict';

let projects = [];
let taskViews = [];
let currentProjectName = null;
let editedProjectName = null;

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
    if (!projects.some(p => p.name === currentProjectName)) {
      currentProjectName = projects.length ? projects[0].name : null;
    }
    renderProjectList();
    await renderProjectDetail();
  } catch (error) { displayError(error); }
}

function renderProjectList() {
  const target = byId('project-list');
  target.replaceChildren();
  byId('project-count').textContent = projects.length;
  if (!projects.length) {
    target.append(el('p', 'pf-project-muted', '还没有项目，点击「新建项目」开始。'));
    return;
  }
  projects.forEach(project => {
    const button = el('button', 'pf-project-entry');
    button.type = 'button';
    button.setAttribute('aria-selected', String(project.name === currentProjectName));
    button.setAttribute('aria-label', '查看 ' + project.name);
    const left = el('span');
    left.append(el('strong', '', project.name));
    left.append(el('small', '', project.root));
    button.append(left, el('span', 'material-symbols-outlined', 'folder'));
    button.addEventListener('click', async () => {
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

async function renderProjectDetail() {
  const placeholder = byId('project-empty'), target = byId('project-detail');
  placeholder.hidden = Boolean(currentProjectName);
  target.hidden = !currentProjectName;
  if (!currentProjectName) return;
  const selected = currentProjectName;
  const data = await requestJson('/api/projects/' + encodeURIComponent(selected));
  if (currentProjectName !== selected) return;
  const project = data.project, files = data.files || [];
  byId('project-current-name').textContent = project.name;
  byId('project-root-preview').textContent = project.root;
  byId('project-job-count').textContent = project.tasks.length;
  byId('project-file-count').textContent = files.length;
  byId('project-exists-count').textContent = files.filter(f => f.exists).length;
  byId('project-runs').hidden = true;
  const jobs = byId('project-jobs');
  jobs.replaceChildren();
  if (!project.tasks.length) jobs.append(el('p', 'pf-project-muted', '暂无关联任务。可先在「定时任务」创建，再编辑项目进行关联。'));
  project.tasks.forEach(name => {
    const row = el('div', 'pf-project-task-item');
    const link = el('a', '', name);
    link.href = '/tasks';
    const task = taskViews.find(t => t.name === name);
    const indicator = el('span', 'pf-project-status', task ? task.status + ' · ' + (task.enabled ? '已启用' : '未启用') : '未找到');
    const actions = el('div', 'pf-project-action-row');
    const history = el('button', 'pf-project-secondary', '执行记录');
    history.type = 'button';
    history.addEventListener('click', () => { void showProjectRuns(name); });
    actions.append(indicator, history);
    row.append(link, actions);
    jobs.append(row);
  });
  const output = byId('project-files');
  output.replaceChildren();
  if (!files.length) output.append(el('p', 'pf-project-muted', '尚未配置输入或输出文件。'));
  files.forEach(file => {
    const row = el('div', 'pf-project-file-item');
    const left = el('div');
    const name = el('div', 'file-name');
    name.append(el('span', 'pf-project-file-role', ({input:'输入',output:'输出',output_dir:'自动扫描目录'})[file.role] || '文件'));
    name.append(document.createTextNode(file.path));
    left.append(name);
    if (file.exists) {
      const modified = file.modified_unix ? new Date(file.modified_unix * 1000).toLocaleString() : '—';
      left.append(el('small', '', '大小 ' + formattedBytes(file.size_bytes) + ' · 修改 ' + modified));
    }
    const right = el('span', 'pf-project-status ' + (file.exists ? 'ok' : 'missing'), file.exists ? '存在' : '未找到');
    row.append(left, right);
    output.append(row);
  });
}

function lines(id) {
  return byId(id).value.split(/\r?\n/).map(x => x.trim()).filter(Boolean);
}

function openProjectForm(projectName) {
  editedProjectName = projectName || null;
  const project = projects.find(p => p.name === editedProjectName);
  byId('project-form-title').textContent = project ? '编辑项目' : '新建项目';
  byId('project-name').value = project ? project.name : '';
  byId('project-root').value = project ? project.root : '';
  byId('project-inputs').value = project ? project.inputs.join('\n') : '';
  byId('project-outputs').value = project ? project.outputs.join('\n') : '';
  byId('project-output-dirs').value = project ? (project.output_dirs || []).join('\n') : '';
  byId('project-form-error').hidden = true;
  const picker = byId('project-task-options');
  picker.replaceChildren();
  if (!taskViews.length) picker.append(el('p','pf-project-muted','尚无任务。请先在「定时任务」页面添加 Python 启动任务。'));
  taskViews.forEach(task => {
    const assigned = projects.find(p => p.name !== editedProjectName && p.tasks.includes(task.name));
    const label = el('label');
    const input = document.createElement('input');
    input.type = 'checkbox';
    input.value = task.name;
    input.checked = Boolean(project && project.tasks.includes(task.name));
    input.disabled = Boolean(assigned);
    label.append(input, document.createTextNode(task.name + (assigned ? '（已属于 ' + assigned.name + '）' : '')));
    picker.append(label);
  });
  byId('project-modal').hidden = false;
  byId('project-name').focus();
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
    output_dirs:lines('project-output-dirs')
  };
  if (!name || !root) {
    errorBox.textContent = '请填写项目名称和完整工作目录。';
    errorBox.hidden = false;
    return;
  }
  if (payload.output_dirs.length > 8) {
    errorBox.textContent = '最多配置 8 个输出目录。';
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

async function deleteProject() {
  if (!currentProjectName) return;
  const name = currentProjectName;
  if (!window.confirm('删除项目「' + name + '」的管理关联？不会删除关联任务和任何文件。')) return;
  try {
    await requestJson('/api/projects/' + encodeURIComponent(name), 'DELETE');
    await loadProjects(null);
  } catch (error) { displayError(error); }
}

async function showProjectRuns(name) {
  const panel = byId('project-runs');
  const output = byId('project-runs-list');
  byId('project-runs-title').textContent = name + ' · 最近执行';
  panel.hidden = false;
  output.replaceChildren(el('p', 'pf-project-muted', '正在加载执行记录…'));
  byId('project-run-changes').replaceChildren();
  try {
    const response = await requestJson('/api/tasks/' + encodeURIComponent(name) + '/history');
    output.replaceChildren();
    const runs = (response.runs || []).slice(0, 12);
    if (!runs.length) output.append(el('p', 'pf-project-muted', '暂无运行记录。'));
    for (const run of runs) {
      const button = el('button', 'pf-project-run-item',
        (run.started_at || '—') + ' · ' + (run.status || '—') + ' · 文件变化');
      button.type = 'button';
      button.addEventListener('click', () => { void showRunFiles(name, run.run_id); });
      output.append(button);
    }
    if (runs.length) await showRunFiles(name, runs[0].run_id);
  } catch (error) { output.replaceChildren(el('p', 'pf-project-form-error', error.message || String(error))); }
}

async function showRunFiles(name, runId) {
  const target = byId('project-run-changes');
  target.replaceChildren(el('p', 'pf-project-muted', '正在读取文件变化…'));
  try {
    const data = await requestJson('/api/tasks/' + encodeURIComponent(name) + '/files/' + encodeURIComponent(runId));
    const record = data.record;
    target.replaceChildren();
    target.append(el('p', 'pf-project-muted', '检查了 ' + record.scanned_paths + ' 条文件/目录元数据（含自动发现）；不包含文件正文。'));
    if (record.incomplete_scan) target.append(el('p', 'pf-project-form-error', '扫描未完整完成（文件数量/目录深度或访问权限限制），此结果可能不包含所有变化。'));
    if (!record.changes.length) target.append(el('p', 'pf-project-muted', '没有检测到已登记文件的变化。'));
    for (const entry of record.changes) {
      const row = el('div', 'pf-project-file-item');
      const name = el('div', 'file-name');
      name.append(el('span', 'pf-project-file-role', entry.role === 'input' ? '输入' : entry.role === 'output_dir' ? '输出目录' : (entry.after?.discovered || entry.before?.discovered) ? '自动发现' : '输出'));
      name.append(document.createTextNode(entry.path));
      const label = {created:'新增',modified:'修改',removed:'删除'}[entry.change] || entry.change;
      const detail = el('span', 'pf-project-status', label);
      row.append(name, detail);
      target.append(row);
    }
  } catch (error) {
    target.replaceChildren(el('p', 'pf-project-muted', '该次执行没有可用的文件变化记录（例如旧记录或未关联项目）。'));
  }
}

function openPythonForm() {
  if (!currentProjectName) return;
  byId('project-python-form').reset();
  byId('project-python-cron').value = '0 0 0 * * *';
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
    errorBox.textContent = '运行超时时间必须是非负整数。';
    errorBox.hidden = false;
    return;
  }
  const body = {
    name:byId('project-python-name').value.trim(),
    python:byId('project-python-exe').value.trim(),
    script:byId('project-python-script').value.trim(),
    args:lines('project-python-args'),
    schedule:byId('project-python-cron').value.trim(), timeout_secs:timeout
  };
  const button = byId('project-python-submit');
  button.disabled = true;
  try {
    const result = await requestJson('/api/projects/' + encodeURIComponent(currentProjectName) + '/python-task', 'POST', body);
    if (result.enabled !== false) throw new Error('安全检查失败：任务不应自动启用');
    closePythonForm();
    await loadProjects(currentProjectName);
  } catch (error) {
    errorBox.textContent = error.message || String(error);
    errorBox.hidden = false;
  } finally { button.disabled = false; }
}

document.addEventListener('DOMContentLoaded', () => {
  byId('project-new').addEventListener('click', () => openProjectForm(null));
  byId('project-add-python').addEventListener('click', openPythonForm);
  byId('project-python-form').addEventListener('submit', savePythonTask);
  byId('project-python-close').addEventListener('click', closePythonForm);
  byId('project-python-cancel').addEventListener('click', closePythonForm);
  byId('project-python-modal').addEventListener('click', e => {
    if (e.target === byId('project-python-modal')) closePythonForm();
  });
  byId('project-runs-close').addEventListener('click', () => { byId('project-runs').hidden = true; });
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
});
