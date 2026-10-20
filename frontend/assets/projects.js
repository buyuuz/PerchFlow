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
  const jobs = byId('project-jobs');
  jobs.replaceChildren();
  if (!project.tasks.length) jobs.append(el('p', 'pf-project-muted', '暂无关联任务。可先在「定时任务」创建，再编辑项目进行关联。'));
  project.tasks.forEach(name => {
    const row = el('div', 'pf-project-task-item');
    const link = el('a', '', name);
    link.href = '/tasks';
    const task = taskViews.find(t => t.name === name);
    const indicator = el('span', 'pf-project-status', task ? task.status + ' · ' + (task.enabled ? '已启用' : '未启用') : '未找到');
    row.append(link, indicator);
    jobs.append(row);
  });
  const output = byId('project-files');
  output.replaceChildren();
  if (!files.length) output.append(el('p', 'pf-project-muted', '尚未配置输入或输出文件。'));
  files.forEach(file => {
    const row = el('div', 'pf-project-file-item');
    const left = el('div');
    const name = el('div', 'file-name');
    name.append(el('span', 'pf-project-file-role', file.role === 'input' ? '输入' : '输出'));
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
    inputs:lines('project-inputs'), outputs:lines('project-outputs')
  };
  if (!name || !root) {
    errorBox.textContent = '请填写项目名称和完整工作目录。';
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

document.addEventListener('DOMContentLoaded', () => {
  byId('project-new').addEventListener('click', () => openProjectForm(null));
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
  });
  void loadProjects();
});
