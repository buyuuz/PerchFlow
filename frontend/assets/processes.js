const STATUS_STYLE = {
  running: { label: 'RUNNING', icon: 'terminal' },
  stopped: { label: 'STOPPED', icon: 'terminal' },
  crashed: { label: 'CRASHED', icon: 'terminal' },
  restarting: { label: 'RESTARTING', icon: 'terminal' },
};

// These are process-page-only strings. Keeping them here preserves the
// existing i18n.js and its other pages unchanged.
const TABLE_TEXT = {
  zh: { name: '进程名称', status: '状态', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'UPTIME', restarts: 'RESTARTS', cmd: '启动命令', actions: '操作', empty: '没有匹配的进程', chooseAll: '选择所有显示的进程', copy: '复制命令', web: '打开网页', edit: '编辑', remove: '删除', selected: '已选择 {count} 项', clear: '取消选择', scroll: '左右滑动表格可查看更多列和操作' },
  en: { name: 'Process name', status: 'Status', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'UPTIME', restarts: 'RESTARTS', cmd: 'Start command', actions: 'Actions', empty: 'No matching processes', chooseAll: 'Select all visible processes', copy: 'Copy command', web: 'Open webpage', edit: 'Edit', remove: 'Delete', selected: '{count} selected', clear: 'Clear selection', scroll: 'Scroll horizontally to view all columns and actions' },
  es: { name: 'Proceso', status: 'Estado', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'TIEMPO', restarts: 'REINICIOS', cmd: 'Comando', actions: 'Acciones', empty: 'No hay procesos coincidentes', chooseAll: 'Seleccionar todos los procesos visibles', copy: 'Copiar comando', web: 'Abrir sitio', edit: 'Editar', remove: 'Eliminar', selected: '{count} seleccionados', clear: 'Borrar selección', scroll: 'Desliza horizontalmente para ver las demás columnas' },
  ja: { name: 'プロセス名', status: '状態', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: '稼働時間', restarts: '再起動', cmd: '起動コマンド', actions: '操作', empty: '該当するプロセスがありません', chooseAll: '表示中のプロセスをすべて選択', copy: 'コマンドをコピー', web: 'Webサイトを開く', edit: '編集', remove: '削除', selected: '{count} 件選択中', clear: '選択解除', scroll: '横にスクロールして他の列を表示' },
  pt: { name: 'Processo', status: 'Estado', pid: 'PID', cpu: 'CPU', mem: 'MEM', uptime: 'TEMPO', restarts: 'REINÍCIOS', cmd: 'Comando', actions: 'Ações', empty: 'Nenhum processo encontrado', chooseAll: 'Selecionar todos os processos visíveis', copy: 'Copiar comando', web: 'Abrir site', edit: 'Editar', remove: 'Excluir', selected: '{count} selecionados', clear: 'Limpar seleção', scroll: 'Role horizontalmente para ver todas as colunas' },
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
  const webpage = processWebUrl(p);
  const checked = selectedProcesses.has(p.name);
  const statusText = STATUS_STYLE[status].label;

  return `
<tr class="${checked ? 'pf-is-selected' : ''}" data-process-name="${escapeHtml(p.name)}">
  <td><input type="checkbox" class="pf-select-process" data-name="${escapeHtml(p.name)}" aria-label="${escapeHtml(p.name)}" ${checked ? 'checked' : ''}></td>
  <td>
    <div class="pf-process-identity">
      <span class="pf-process-icon pf-process-icon--${status}" aria-hidden="true"><span class="material-symbols-outlined">${processIcon(p.name)}</span></span>
      <div class="pf-process-name-stack">
        <a class="pf-process-name" href="/process-detail?name=${encodedName}" title="${escapeHtml(p.name)}">${escapeHtml(p.name)}</a>
        ${String(p.alias_zh || '').trim() ? `<span class="pf-process-alias" title="${escapeHtml(p.alias_zh)}">${escapeHtml(p.alias_zh)}</span>` : ''}
      </div>
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
      ${actionButton(p, 'web', 'open_in_new', webpage ? tableText('web') : tableText('web') + '（未识别网页地址）', !running || !webpage, 'pf-action--terminal')}
    </div>
  </td>
</tr>`;
}

let cachedProcesses = [];
const autoWebCache = new Map();
let editingSettings = null;
function processWebUrl(p) {
  if (p.web_url) return p.web_url;
  const found = autoWebCache.get(p.name);
  return found && found.pid === p.pid ? found.url : null;
}
async function discoverWebProcess(p) {
  const active=p.status==='running'||p.status==='restarting';
  if(!active||!p.pid||p.web_url) return;
  const previous=autoWebCache.get(p.name);
  const now=Date.now();
  if(previous&&previous.pid===p.pid&&(previous.url||previous.inflight||previous.attempts>=5||now<previous.nextAt)) return;
  const entry=previous&&previous.pid===p.pid?previous:{pid:p.pid,attempts:0,url:null,candidates:[],nextAt:0};
  entry.inflight=true;entry.attempts++;
  autoWebCache.set(p.name,entry);
  try {
    const res=await apiJson('/api/processes/'+encodeURIComponent(p.name)+'/web');
    if(res.ok){
      entry.url=res.url||null;
      entry.candidates=Array.isArray(res.candidates)?res.candidates:[];
    }
  } catch(error) { console.warn('Web port discovery:',p.name,error); }
  finally {
    entry.inflight=false;
    entry.nextAt=Date.now()+Math.min(30000,3000*entry.attempts);
    if(autoWebCache.get(p.name)===entry)renderProcessTable();
  }
}
function showWebMode() {
  const isManual=document.getElementById('add-web-mode').value==='manual';
  document.getElementById('add-web-url').hidden=!isManual;
  document.getElementById('add-web-hint').textContent=isManual
    ?'手动地址优先，自动检测不会覆盖。':'自动识别进程及子进程的本地 Web 端口。';
}
function setWebMode(url) {
  document.getElementById('add-web-mode').value=url?'manual':'auto';
  document.getElementById('add-web-url').value=url||'';
  showWebMode();
}
function addEnvRow(key='',value='') {
  const row=document.createElement('div');row.className='pf-process-env-row';
  const keyEl=document.createElement('input');keyEl.type='text';keyEl.value=key;
  keyEl.placeholder='VARIABLE_NAME';keyEl.className='pf-process-field';keyEl.setAttribute('aria-label','环境变量名称');
  const valueEl=document.createElement('input');
  valueEl.type=/TOKEN|PASSWORD|SECRET|API_KEY|PRIVATE_KEY|CREDENTIAL/i.test(key)?'password':'text';
  valueEl.value=value;valueEl.placeholder='变量值';valueEl.className='pf-process-field';valueEl.setAttribute('aria-label','环境变量值');
  keyEl.addEventListener('input',()=>{
    const secret=/TOKEN|PASSWORD|SECRET|API_KEY|PRIVATE_KEY|CREDENTIAL/i.test(keyEl.value);
    valueEl.type=secret?'password':'text';
  });
  const remove=document.createElement('button');remove.type='button';remove.textContent='×';remove.title='删除该环境变量';
  remove.addEventListener('click',()=>row.remove());
  row.append(keyEl,valueEl,remove);
  document.getElementById('add-env-vars').append(row);
}
function configureAdvanced(settings={}) {
  const vars=document.getElementById('add-env-vars');vars.replaceChildren();
  const env=settings.env||{};
  for(const key of Object.keys(env).sort())addEnvRow(key,env[key]);
  const restart=settings.restart||{};
  document.getElementById('add-restart-policy').value=restart.policy||'on-failure';
  document.getElementById('add-restart-backoff').value=restart.backoff_secs??2;
  document.getElementById('add-restart-max').value=restart.backoff_max_secs??60;
  document.getElementById('add-restart-retries').value=restart.max_retries??0;
  document.getElementById('add-advanced').open=false;
}
function advancedPayload(){
  const env={};
  for(const row of document.querySelectorAll('#add-env-vars .pf-process-env-row')){
    const [keyInput,valueInput]=row.querySelectorAll('input');
    const key=keyInput.value.trim();
    if(!key || /[=\x00]/.test(key))throw Error('环境变量名称不能为空，也不能包含 =');
    if(Object.hasOwn(env,key))throw Error('环境变量名称不能重复：'+key);
    env[key]=valueInput.value;
  }
  const number=(id,min,max)=>{
    const raw=document.getElementById(id).value;
    const n=Number(raw);
    if(!raw||!Number.isInteger(n)||n<min||n>max)throw Error('重启参数必须是有效的整数');
    return n;
  };
  const restart={
    policy:document.getElementById('add-restart-policy').value,
    backoff_secs:number('add-restart-backoff',1,86400),
    backoff_max_secs:number('add-restart-max',1,86400),
    max_retries:number('add-restart-retries',0,100000)
  };
  if(restart.backoff_max_secs<restart.backoff_secs)throw Error('最大重试延迟不能小于初始延迟');
  return {env,restart};
}
const sameJson=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const sortedEnv=env=>Object.fromEntries(Object.entries(env||{}).sort(([a],[b])=>a.localeCompare(b)));

let editingName = null;
let processSearch = '';
let sortKey = null;
let sortAscending = true;
const selectedProcesses = new Set();

function getVisibleProcesses() {
  const needle = processSearch.trim().toLocaleLowerCase();
  const filtered = cachedProcesses.filter(p => (
    !needle || String(p.name || '').toLocaleLowerCase().includes(needle)
    || String(p.alias_zh || '').toLocaleLowerCase().includes(needle)
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
    for(const p of cachedProcesses)void discoverWebProcess(p);
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
  const saveButton = document.querySelector('#add-modal button[onclick="submitAddModal()"]');
  saveButton.setAttribute('data-i18n', 'button.add_process');
  saveButton.textContent = t('button.add_process');
  document.getElementById('add-name').value = '';
  document.getElementById('add-alias-zh').value = '';
  document.getElementById('add-name').disabled = false;
  document.getElementById('add-cmd').value = '';
  document.getElementById('add-args').value = '';
  resetWorkingDirectoryField();
  document.getElementById('add-autostart').checked = false;
  editingSettings=null;setWebMode(null);configureAdvanced();
  document.getElementById('add-err').classList.add('hidden');
  showAddModal();
}

async function editProcess(name) {
  const p = cachedProcesses.find(x => x.name === decodeURIComponent(name));
  if (!p) return;
  // Fetch advanced values only for the requested process, never poll secrets in the list.
  let settings;
  try {
    settings=await apiJson('/api/processes/'+encodeURIComponent(p.name)+'/settings');
  }catch(error){
    await PerchDialog.notice('无法读取高级配置，未打开编辑窗口，防止覆盖已有环境变量。');
    return;
  }
  editingSettings=settings;
  editingName = p.name;
  const title = document.getElementById('add-modal-title');
  title.removeAttribute('data-i18n');
  title.textContent = t('modal.edit_process');
  const saveButton = document.querySelector('#add-modal button[onclick="submitAddModal()"]');
  saveButton.setAttribute('data-i18n', 'button.save_changes');
  saveButton.textContent = t('button.save_changes');
  document.getElementById('add-name').value = p.name;
  document.getElementById('add-alias-zh').value = p.alias_zh || '';
  document.getElementById('add-name').disabled = true;
  document.getElementById('add-cmd').value = p.cmd_raw;
  document.getElementById('add-args').value = (p.args || []).join(' ');
  resetWorkingDirectoryField(p.cwd || '');
  document.getElementById('add-autostart').checked = p.autostart;
  setWebMode(settings.web_url||null);
  configureAdvanced(settings);
  document.getElementById('add-err').classList.add('hidden');
  showAddModal();
}

async function deleteProcess(name) {
  if (!await PerchDialog.confirm(t('confirm.delete_process'),{title:'删除进程',danger:true})) return;
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
  const alias_zh = document.getElementById('add-alias-zh').value.trim();
  // Windows CreateProcess expects the executable path without surrounding quotes.
  const cmd = normalizedCwdText(document.getElementById('add-cmd').value);
  const args = document.getElementById('add-args').value.trim().split(/\s+/).filter(Boolean);
  const cwd = getWorkingDirectoryInput();
  const autostart = document.getElementById('add-autostart').checked;
  const webMode=document.getElementById('add-web-mode').value;
  const web_url=webMode==='manual'?document.getElementById('add-web-url').value.trim():null;
  const errEl = document.getElementById('add-err');
  const saveButton = document.querySelector('#add-modal button[onclick="submitAddModal()"]');
  errEl.classList.add('hidden');

  if (Array.from(alias_zh).length > 80) {
    errEl.textContent = t('error.alias_too_long');
    errEl.classList.remove('hidden');
    return;
  }
  if(webMode==='manual'&&!/^https?:\/\/[^@\s]+$/i.test(web_url)){
    errEl.textContent='网页地址必须是 http:// 或 https:// 开头的完整地址。';
    errEl.classList.remove('hidden');return;
  }
  let advanced;
  try{advanced=advancedPayload();}catch(error){
    errEl.textContent=error.message;errEl.classList.remove('hidden');return;
  }
  if (!name || !cmd) {
    errEl.textContent = t('error.name_cmd_required');
    errEl.classList.remove('hidden');
    return;
  }

  const old = editingName ? cachedProcesses.find(p => p.name === editingName) : null;
  const sameRuntimeSettings = Boolean(old) && editingSettings &&
    sameJson(sortedEnv(advanced.env),sortedEnv(editingSettings.env)) &&
    sameJson(advanced.restart,editingSettings.restart) &&
    cmd === normalizedCwdText(old.cmd_raw) &&
    JSON.stringify(args) === JSON.stringify(old.args || []) &&
    autostart === Boolean(old.autostart);
  const cwdChanged = Boolean(old) && cwd !== String(old.cwd || '').trim();
  const aliasChanged = Boolean(old) && alias_zh !== String(old.alias_zh || '').trim();
  const webChanged=Boolean(old)&&web_url!==String(editingSettings?.web_url||'').trim()&&(web_url!==null||editingSettings.web_url!=null);
  const runtimeChanged=Boolean(old)&&(!sameRuntimeSettings||cwdChanged);

  // Save only an alias if that is all that changed; it never restarts processes.
  // A cwd-only edit uses the dedicated endpoint to preserve hidden env/restart.
  // Full PUT is required only for changes to execution settings.
  if ((!old || !sameRuntimeSettings || cwdChanged) && !(await validateWorkingDirectory())) {
    document.getElementById('add-cwd').focus();
    return;
  }

  const save = async (path, method, body) => {
    const resp = await api(path, { method, body: JSON.stringify(body) });
    const result = await resp.json();
    if (!resp.ok || !result.ok) {
      const errors = {
        cwd_too_long: 'error.cwd_too_long',
        cwd_not_absolute: 'error.cwd_not_absolute',
        cwd_not_found: 'error.cwd_not_found',
        cwd_not_directory: 'error.cwd_not_directory',
        cwd_inaccessible: 'error.cwd_inaccessible',
      };
      throw new Error(errors[result.error] ? t(errors[result.error]) : (result.error || t('error.save_failed')));
    }
  };

  if(runtimeChanged&&(old.status==='running'||old.status==='restarting')){
    const agreed=await PerchDialog.confirm('修改后的启动配置需要重启进程才能生效。现在保存并重启？',
      {title:'重启以应用设置',ok:'保存并重启'});
    if(!agreed)return;
  }
  if (saveButton) saveButton.disabled = true;
  try {
    if (old && sameRuntimeSettings) {
      const base = '/api/processes/' + encodeURIComponent(editingName);
      if (cwdChanged) {
        await save(base + '/cwd', 'PUT', { cwd: cwd || null });
      }
      if (aliasChanged) {
        await save(base + '/alias', 'PUT', { alias_zh: alias_zh || null });
      }
      if (webChanged) await save(base + '/web-url','PUT',{web_url});
    } else {
      const body = {
        name, alias_zh: alias_zh || null, cmd, args, cwd: cwd || null, autostart, web_url, ...advanced,
      };
      const path = editingName
        ? '/api/processes/' + encodeURIComponent(editingName)
        : '/api/processes';
      await save(path, editingName ? 'PUT' : 'POST', body);
    }
    closeAddModal();
    await loadProcesses();
  } catch (err) {
    errEl.textContent = err.message || t('error.save_failed');
    errEl.classList.remove('hidden');
    // An alias/cwd multi-step update can be partially saved if a later request
    // fails. Reload from the server so the next edit shows the persisted state.
    await loadProcesses();
  } finally {
    if (saveButton) saveButton.disabled = false;
  }
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
      else if (action === 'web'){
        const p=cachedProcesses.find(p=>p.name===name);
        const url=p&&processWebUrl(p);
        if(url&&(p.status==='running'||p.status==='restarting')&&/^https?:\/\//i.test(url)){
          window.open(url,'_blank','noopener,noreferrer');
        }
      }
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
  document.getElementById('add-web-mode').addEventListener('change',showWebMode);
  document.getElementById('add-env-add').addEventListener('click',()=>addEnvRow());
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
