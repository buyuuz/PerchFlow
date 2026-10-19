const STATUS_STYLE = {
  running: { label: 'RUNNING', icon: 'terminal' },
  stopped: { label: 'STOPPED', icon: 'terminal' },
  crashed: { label: 'CRASHED', icon: 'terminal' },
  restarting: { label: 'RESTARTING', icon: 'terminal' },
};

// These are process-page-only strings. Keeping them here preserves the
// existing i18n.js and its other pages unchanged.
const TABLE_TEXT = {
  zh: { name: '进程名称', status: '状态', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'UPTIME', restarts: 'RESTARTS', cmd: '启动命令', actions: '操作', empty: '没有匹配的进程', chooseAll: '选择所有显示的进程', copy: '复制命令', logs: '查看日志', edit: '编辑', remove: '删除', selected: '已选择 {count} 项', clear: '取消选择', scroll: '左右滑动表格可查看更多列和操作' },
  en: { name: 'Process name', status: 'Status', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'UPTIME', restarts: 'RESTARTS', cmd: 'Start command', actions: 'Actions', empty: 'No matching processes', chooseAll: 'Select all visible processes', copy: 'Copy command', logs: 'View logs', edit: 'Edit', remove: 'Delete', selected: '{count} selected', clear: 'Clear selection', scroll: 'Scroll horizontally to view all columns and actions' },
  es: { name: 'Proceso', status: 'Estado', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'TIEMPO', restarts: 'REINICIOS', cmd: 'Comando', actions: 'Acciones', empty: 'No hay procesos coincidentes', chooseAll: 'Seleccionar todos los procesos visibles', copy: 'Copiar comando', logs: 'Ver registros', edit: 'Editar', remove: 'Eliminar', selected: '{count} seleccionados', clear: 'Borrar selección', scroll: 'Desliza horizontalmente para ver las demás columnas' },
  ja: { name: 'プロセス名', status: '状態', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: '稼働時間', restarts: '再起動', cmd: '起動コマンド', actions: '操作', empty: '該当するプロセスがありません', chooseAll: '表示中のプロセスをすべて選択', copy: 'コマンドをコピー', logs: 'ログを表示', edit: '編集', remove: '削除', selected: '{count} 件選択中', clear: '選択解除', scroll: '横にスクロールして他の列を表示' },
  pt: { name: 'Processo', status: 'Estado', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'TEMPO', restarts: 'REINÍCIOS', cmd: 'Comando', actions: 'Ações', empty: 'Nenhum processo encontrado', chooseAll: 'Selecionar todos os processos visíveis', copy: 'Copiar comando', logs: 'Ver logs', edit: 'Editar', remove: 'Excluir', selected: '{count} selecionados', clear: 'Limpar seleção', scroll: 'Role horizontalmente para ver todas as colunas' },
};

function tableText(key, params = {}) {
  const lang = typeof getLang === 'function' ? getLang() : 'zh';
  let value = (TABLE_TEXT[lang] || TABLE_TEXT.en)[key] || TABLE_TEXT.en[key] || key;
  for (const [k, v] of Object.entries(params)) value = value.replace('{' + k + '}', String(v));
  return value;
}

function escapeHtml(s) {
  return String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

function fmtUptime(value) {
  const secs = Math.max(0, Math.floor(Number(value) || 0));
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m ${secs % 60}s`;
}

function processIcon(name) {
  const s = String(name || '').toLowerCase();
  if (s.includes('mail')) return 'mail';
  if (s.includes('daily') || s.includes('check') || s.includes('sched')) return 'calendar_month';
  if (s.includes('video') || s.includes('publish')) return 'videocam';
  if (s.includes('data') || s.includes('collect') || s.includes('sync')) return 'database';
  return 'terminal';
}

function actionButton(p, action, icon, title, disabled = false, extraClass = '') {
  return `<button type="button" class="pf-action ${extraClass}" data-action="${action}" data-name="${escapeHtml(p.name)}" title="${escapeHtml(title)}" aria-label="${escapeHtml(title + ': ' + p.name)}"${disabled ? ' disabled' : ''}><span class="material-symbols-outlined" aria-hidden="true">${icon}</span></button>`;
}

function renderProcessRow(p) {
  const status = Object.hasOwn(STATUS_STYLE, p.status) ? p.status : 'stopped';
  const cpu = Number.isFinite(Number(p.cpu_percent)) ? Number(p.cpu_percent).toFixed(1) : '0.0';
  const memMb = Math.round(Math.max(0, Number(p.mem_bytes) || 0) / 1048576);
  const pidText = p.pid != null ? String(p.pid) : '---';
  const command = String(p.cmd || '');
  const encodedName = encodeURIComponent(p.name);
  const running = status === 'running' || status === 'restarting';
  const checked = selectedProcesses.has(p.name);
  const statusText = STATUS_STYLE[status].label;

  return `
<tr class="${checked ? 'pf-is-selected' : ''}" data-process-name="${escapeHtml(p.name)}">
  <td><input type="checkbox" class="pf-select-process" data-name="${escapeHtml(p.name)}" aria-label="${escapeHtml(p.name)}" ${checked ? 'checked' : ''}></td>
  <td>
    <div class="pf-process-identity">
      <span class="pf-process-icon pf-process-icon--${status}" aria-hidden="true"><span class="material-symbols-outlined">${processIcon(p.name)}</span></span>
      <a class="pf-process-name" href="/process-detail?name=${encodedName}" title="${escapeHtml(p.name)}">${escapeHtml(p.name)}</a>
    </div>
  </td>
  <td><span class="pf-status pf-status--${status}" title="${escapeHtml(typeof t === 'function' ? t('status.' + status) : statusText)}">${statusText}</span></td>
  <td class="pf-process-pid">${escapeHtml(pidText)}</td>
  <td class="pf-process-metric">${cpu}%</td>
  <td class="pf-process-metric">${memMb} MB</td>
  <td class="pf-process-metric">${fmtUptime(p.uptime_secs)}</td>
  <td class="pf-process-metric">${Math.max(0, Number(p.restart_count) || 0)}</td>
  <td>
    <div class="pf-command" title="${escapeHtml(command)}">
      <span class="pf-command-dollar" aria-hidden="true">$</span>
      <span class="pf-command-text">${escapeHtml(command)}</span>
      <button type="button" class="pf-action" data-action="copy" data-name="${escapeHtml(p.name)}" title="${escapeHtml(tableText('copy'))}" aria-label="${escapeHtml(tableText('copy') + ': ' + p.name)}"><span class="material-symbols-outlined" aria-hidden="true">content_copy</span></button>
    </div>
  </td>
  <td>
    <div class="pf-actions">
      ${actionButton(p, 'start', 'play_arrow', t('button.start'), running)}
      ${actionButton(p, 'restart', 'refresh', t('button.restart'))}
      ${actionButton(p, 'stop', 'stop', t('button.stop'), !running)}
      ${actionButton(p, 'edit', 'edit', tableText('edit'))}
      ${actionButton(p, 'delete', 'delete', tableText('remove'), false, 'pf-action--delete')}
      ${actionButton(p, 'logs', 'terminal', tableText('logs'), false, 'pf-action--terminal')}
    </div>
  </td>
</tr>`;
}

let cachedProcesses = [];
let editingName = null;
let processSearch = '';
let sortKey = null;
let sortAscending = true;
const selectedProcesses = new Set();

function getVisibleProcesses() {
  const needle = processSearch.trim().toLocaleLowerCase();
  const filtered = cachedProcesses.filter(p => (
    !needle || String(p.name || '').toLocaleLowerCase().includes(needle)
    || String(p.cmd || '').toLocaleLowerCase().includes(needle)
    || String(p.status || '').toLocaleLowerCase().includes(needle)
    || (p.pid != null && String(p.pid).includes(needle))
  ));
  if (sortKey) {
    filtered.sort((a, b) => {
      const valueFor = (p) => {
        if (sortKey === 'name') return String(p.name || '');
        if (sortKey === 'status') return String(p.status || '');
        if (sortKey === 'pid') return Number(p.pid ?? -1);
        if (sortKey === 'cpu') return Number(p.cpu_percent || 0);
        if (sortKey === 'mem') return Number(p.mem_bytes || 0);
        if (sortKey === 'uptime') return Number(p.uptime_secs || 0);
        if (sortKey === 'restarts') return Number(p.restart_count || 0);
        return '';
      };
      const x = valueFor(a), y = valueFor(b);
      const result = typeof x === 'string' ? x.localeCompare(y) : x - y;
      return sortAscending ? result : -result;
    });
  }
  return filtered;
}

function renderProcessTable() {
  const rows = getVisibleProcesses();
  const body = document.getElementById('proc-table-body');
  if (!body) return;
  body.innerHTML = rows.length
    ? rows.map(renderProcessRow).join('')
    : `<tr><td colspan="10" class="pf-process-empty">${escapeHtml(tableText('empty'))}</td></tr>`;

  document.querySelectorAll('[data-table-label]').forEach(el => {
    el.textContent = tableText(el.dataset.tableLabel);
  });
  document.querySelectorAll('[data-sort-key]').forEach(el => {
    const key = el.dataset.sortKey;
    const th = el.closest('th');
    if (th) th.setAttribute('aria-sort', key === sortKey ? (sortAscending ? 'ascending' : 'descending') : 'none');
    const icon = el.querySelector('.pf-sort-icon');
    if (icon) icon.textContent = key === sortKey ? (sortAscending ? 'arrow_upward' : 'arrow_downward') : 'unfold_more';
  });

  const selectAll = document.getElementById('proc-select-all');
  if (selectAll) {
    const chosen = rows.filter(p => selectedProcesses.has(p.name)).length;
    selectAll.checked = rows.length > 0 && chosen === rows.length;
    selectAll.indeterminate = chosen > 0 && chosen < rows.length;
    selectAll.setAttribute('aria-label', tableText('chooseAll'));
  }
  const toolbar = document.getElementById('proc-selection-toolbar');
  if (toolbar) toolbar.hidden = selectedProcesses.size === 0;
  const selectedLabel = document.getElementById('proc-selected-label');
  if (selectedLabel) selectedLabel.textContent = tableText('selected', { count: selectedProcesses.size });
  const clearButton = document.getElementById('proc-clear-selection');
  if (clearButton) clearButton.textContent = tableText('clear');
  const hint = document.getElementById('proc-scroll-hint');
  if (hint) hint.textContent = tableText('scroll');
}

async function loadProcesses() {
  try {
    const result = await apiJson('/api/processes');
    cachedProcesses = Array.isArray(result.processes) ? result.processes : [];
    const names = new Set(cachedProcesses.map(p => p.name));
    for (const name of selectedProcesses) if (!names.has(name)) selectedProcesses.delete(name);
    renderProcessTable();
  } catch (err) {
    console.error('Failed to refresh processes', err);
    if (!cachedProcesses.length) {
      const body = document.getElementById('proc-table-body');
      if (body) body.innerHTML = '<tr><td class="pf-process-empty" colspan="10">Failed to load processes</td></tr>';
    }
  }
}

async function copyProcessCommand(name, button) {
  const p = cachedProcesses.find(x => x.name === name);
  if (!p) return;
  const value = String(p.cmd || '');
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(value);
    } else {
      const field = document.createElement('textarea');
      field.value = value;
      field.style.position = 'fixed';
      field.style.opacity = '0';
      document.body.appendChild(field);
      field.select();
      const ok = document.execCommand('copy');
      field.remove();
      if (!ok) throw new Error('Clipboard unavailable');
    }
    button.title = 'Copied';
    setTimeout(() => { if (button.isConnected) button.title = tableText('copy'); }, 1300);
  } catch (err) {
    console.error('Copy failed', err);
  }
}

async function act(name, action) {
  await api(`/api/processes/${name}/${action}`, { method: 'POST' });
  await loadProcesses();
}

function openAddModal() {
  editingName = null;
  const title = document.getElementById('add-modal-title');
  title.setAttribute('data-i18n', 'modal.add_process');
  title.textContent = t('modal.add_process');
  document.getElementById('add-name').value = '';
  document.getElementById('add-name').disabled = false;
  document.getElementById('add-cmd').value = '';
  document.getElementById('add-args').value = '';
  document.getElementById('add-autostart').checked = false;
  document.getElementById('add-err').classList.add('hidden');
  showAddModal();
}

function editProcess(name) {
  const p = cachedProcesses.find(x => x.name === decodeURIComponent(name));
  if (!p) return;
  editingName = p.name;
  const title = document.getElementById('add-modal-title');
  title.removeAttribute('data-i18n');
  title.textContent = t('modal.edit_process');
  document.getElementById('add-name').value = p.name;
  document.getElementById('add-name').disabled = true;
  document.getElementById('add-cmd').value = p.cmd_raw;
  document.getElementById('add-args').value = (p.args || []).join(' ');
  document.getElementById('add-autostart').checked = p.autostart;
  document.getElementById('add-err').classList.add('hidden');
  showAddModal();
}

async function deleteProcess(name) {
  if (!confirm(t('confirm.delete_process'))) return;
  await api(`/api/processes/${name}`, { method: 'DELETE' });
  await loadProcesses();
}

function showAddModal() {
  document.getElementById('add-modal').classList.remove('hidden');
  document.getElementById('add-modal').classList.add('flex');
}

function closeAddModal() {
  document.getElementById('add-modal').classList.add('hidden');
  document.getElementById('add-modal').classList.remove('flex');
}

async function submitAddModal() {
  const name = document.getElementById('add-name').value.trim();
  const cmd = document.getElementById('add-cmd').value.trim();
  const args = document.getElementById('add-args').value.trim().split(/\s+/).filter(Boolean);
  const autostart = document.getElementById('add-autostart').checked;
  const errEl = document.getElementById('add-err');
  if (!name || !cmd) {
    errEl.textContent = t('error.name_cmd_required');
    errEl.classList.remove('hidden');
    return;
  }
  // Keep existing cwd on edit; the modal does not expose that field.
  const cwd = editingName ? cachedProcesses.find(x => x.name === editingName)?.cwd : undefined;
  const body = {
    name, cmd, args, cwd, autostart,
    restart: { policy: 'on-failure', backoff_secs: 2, backoff_max_secs: 60, max_retries: 0 },
  };
  const path = editingName ? `/api/processes/${encodeURIComponent(editingName)}` : '/api/processes';
  const method = editingName ? 'PUT' : 'POST';
  const resp = await api(path, { method, body: JSON.stringify(body) });
  const result = await resp.json();
  if (!result.ok) {
    errEl.textContent = result.error || t('error.save_failed');
    errEl.classList.remove('hidden');
    return;
  }
  closeAddModal();
  await loadProcesses();
}

document.addEventListener('DOMContentLoaded', () => {
  const search = document.getElementById('proc-search');
  if (search) search.addEventListener('input', () => {
    processSearch = search.value;
    renderProcessTable();
  });
  const table = document.querySelector('.pf-process-table');
  if (table) {
    table.addEventListener('click', event => {
      const sort = event.target.closest('[data-sort-key]');
      if (sort) {
        const key = sort.dataset.sortKey;
        sortAscending = sortKey === key ? !sortAscending : true;
        sortKey = key;
        renderProcessTable();
        return;
      }
      const button = event.target.closest('button[data-action]');
      if (!button) return;
      const name = button.dataset.name;
      const action = button.dataset.action;
      if (action === 'copy') void copyProcessCommand(name, button);
      else if (action === 'edit') editProcess(encodeURIComponent(name));
      else if (action === 'delete') void deleteProcess(encodeURIComponent(name));
      else if (action === 'logs') window.location.href = '/process-detail?name=' + encodeURIComponent(name);
      else if (['start', 'stop', 'restart'].includes(action)) void act(encodeURIComponent(name), action);
    });
    table.addEventListener('change', event => {
      if (event.target.id === 'proc-select-all') {
        const checked = event.target.checked;
        for (const p of getVisibleProcesses()) {
          if (checked) selectedProcesses.add(p.name);
          else selectedProcesses.delete(p.name);
        }
        renderProcessTable();
        return;
      }
      if (event.target.matches('.pf-select-process')) {
        const name = event.target.dataset.name;
        if (event.target.checked) selectedProcesses.add(name);
        else selectedProcesses.delete(name);
        renderProcessTable();
      }
    });
  }
  const clear = document.getElementById('proc-clear-selection');
  if (clear) clear.addEventListener('click', () => {
    selectedProcesses.clear();
    renderProcessTable();
  });
  // HTML selector invokes setLang() first; update dynamic labels afterwards.
  document.querySelectorAll('[data-lang-select]').forEach(el => el.addEventListener('change', renderProcessTable));
  void loadProcesses();
  setInterval(loadProcesses, 3000);
});
