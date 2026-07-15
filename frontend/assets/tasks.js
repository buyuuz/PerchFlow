let editingName = null;

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

function renderRow(t) {
  const toggleActive = t.enabled ? 'active' : '';
  const lastRun = t.last_run ? new Date(t.last_run).toLocaleString() : '-';
  const statusIcon = t.last_exit_code == null ? 'remove'
    : (t.last_exit_code === 0 ? 'check_circle' : 'error');
  const statusColor = t.last_exit_code == null ? 'text-on-surface-variant'
    : (t.last_exit_code === 0 ? 'text-green-400' : 'text-error');
  return `
<div class="glass-card rounded-lg flex items-center p-3 transition-colors duration-200 group mb-2">
<div class="w-10 flex justify-center">
<div class="w-8 h-4 rounded-full glass-toggle ${toggleActive} relative cursor-pointer" onclick="toggleEnabled('${encodeURIComponent(t.name)}', ${!t.enabled})">
<div class="w-4 h-4 rounded-full glass-toggle-thumb absolute left-0 top-0"></div>
</div>
</div>
<div class="flex-1.5 grid grid-cols-[1.5fr_1fr_2fr_1fr] gap-4 items-center w-full ml-4">
<div class="font-body-md text-body-md font-semibold text-on-surface truncate">${escapeHtml(t.name)}</div>
<div class="font-mono-data text-mono-data text-primary truncate bg-primary/10 px-2 py-0.5 rounded w-max">${escapeHtml(t.schedule)}</div>
<div class="font-mono-data text-mono-data text-on-surface-variant truncate opacity-80">${escapeHtml(t.cmd)}</div>
<div class="flex items-center gap-2">
<span class="material-symbols-outlined text-[16px] ${statusColor}">${statusIcon}</span>
<span class="font-mono-data text-mono-data text-on-surface-variant">${lastRun}</span>
</div>
</div>
<div class="w-24 flex justify-end gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
<button class="p-1.5 text-on-surface-variant hover:text-primary rounded hover:bg-white/5 transition-colors" onclick="runNow('${encodeURIComponent(t.name)}')" title="Run Now">
<span class="material-symbols-outlined text-[18px]">play_arrow</span>
</button>
<button class="p-1.5 text-on-surface-variant hover:text-on-surface rounded hover:bg-white/5 transition-colors" onclick="editTask('${encodeURIComponent(t.name)}')" title="Edit">
<span class="material-symbols-outlined text-[18px]">edit</span>
</button>
<button class="p-1.5 text-on-surface-variant hover:text-error rounded hover:bg-error/10 transition-colors" onclick="deleteTask('${encodeURIComponent(t.name)}')" title="Delete">
<span class="material-symbols-outlined text-[18px]">delete</span>
</button>
</div>
</div>`;
}

let cachedTasks = [];

async function loadTasks() {
  const { tasks } = await apiJson('/api/tasks');
  cachedTasks = tasks;
  document.getElementById('task-rows').innerHTML = tasks.map(renderRow).join('');
  const active = tasks.filter(t => t.enabled).length;
  document.getElementById('task-count').textContent = `Total: ${tasks.length} Active: ${active}`;
}

async function runNow(name) {
  await api(`/api/tasks/${name}/run`, { method: 'POST' });
  loadTasks();
}

async function deleteTask(name) {
  await api(`/api/tasks/${name}`, { method: 'DELETE' });
  loadTasks();
}

async function toggleEnabled(name, enabled) {
  const t = cachedTasks.find(x => x.name === decodeURIComponent(name));
  if (!t) return;
  const body = { name: t.name, cmd: t.cmd, args: t.args || [], cwd: t.cwd, schedule: t.schedule, enabled };
  await api(`/api/tasks/${name}`, { method: 'PUT', body: JSON.stringify(body) });
  loadTasks();
}

function openAddModal() {
  editingName = null;
  document.getElementById('task-modal-title').textContent = '添加任务';
  document.getElementById('task-name').value = '';
  document.getElementById('task-name').disabled = false;
  document.getElementById('task-cmd').value = '';
  document.getElementById('task-args').value = '';
  document.getElementById('task-schedule').value = '';
  document.getElementById('task-enabled').checked = true;
  document.getElementById('task-err').classList.add('hidden');
  showTaskModal();
}

function editTask(name) {
  const t = cachedTasks.find(x => x.name === decodeURIComponent(name));
  if (!t) return;
  editingName = t.name;
  document.getElementById('task-modal-title').textContent = '编辑任务';
  document.getElementById('task-name').value = t.name;
  document.getElementById('task-name').disabled = true;
  document.getElementById('task-cmd').value = t.cmd;
  document.getElementById('task-args').value = (t.args || []).join(' ');
  document.getElementById('task-schedule').value = t.schedule;
  document.getElementById('task-enabled').checked = t.enabled;
  document.getElementById('task-err').classList.add('hidden');
  showTaskModal();
}

function showTaskModal() {
  document.getElementById('task-modal').classList.remove('hidden');
  document.getElementById('task-modal').classList.add('flex');
}

function closeTaskModal() {
  document.getElementById('task-modal').classList.add('hidden');
  document.getElementById('task-modal').classList.remove('flex');
}

async function submitTaskModal() {
  const name = document.getElementById('task-name').value.trim();
  const cmd = document.getElementById('task-cmd').value.trim();
  const args = document.getElementById('task-args').value.trim().split(/\s+/).filter(Boolean);
  const schedule = document.getElementById('task-schedule').value.trim();
  const enabled = document.getElementById('task-enabled').checked;
  const errEl = document.getElementById('task-err');
  if (!name || !cmd || !schedule) {
    errEl.textContent = '名称、命令和 cron 表达式不能为空';
    errEl.classList.remove('hidden');
    return;
  }
  const cwd = editingName ? cachedTasks.find(x => x.name === editingName)?.cwd : undefined;
  const body = { name, cmd, args, cwd, schedule, enabled };
  const path = editingName ? `/api/tasks/${encodeURIComponent(editingName)}` : '/api/tasks';
  const method = editingName ? 'PUT' : 'POST';
  const resp = await api(path, { method, body: JSON.stringify(body) });
  const result = await resp.json();
  if (!result.ok) {
    errEl.textContent = result.error || '保存失败';
    errEl.classList.remove('hidden');
    return;
  }
  closeTaskModal();
  loadTasks();
}

document.addEventListener('DOMContentLoaded', () => { loadTasks(); setInterval(loadTasks, 3000); });
