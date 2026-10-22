/* PerchFlow scheduled tasks — matches the process-management table.
   Never synthesise "next run": that field is not provided by the Rust API. */

const TASK_TEXT = {
  zh: { name:'任务名称', enabled:'状态', schedule:'Cron 表达式', command:'启动命令', last_run:'上次运行', last_exit_code:'运行结果', actions:'操作', on:'已启用', off:'已停用', success:'成功', failed:'失败', never:'尚未运行', run:'立即运行', enable:'启用任务', disable:'停用任务', edit:'编辑', remove:'删除', copy:'复制命令', copied:'已复制', empty:'没有匹配的定时任务', count:'共 {total} 项 · 已启用 {active} 项', selected:'已选择 {count} 项', clear:'取消选择', chooseAll:'选择所有显示的任务', scroll:'左右滑动表格可查看更多列和操作', error:'任务操作失败，请重试' },
  en: { name:'Task name', enabled:'Status', schedule:'Cron schedule', command:'Command', last_run:'Last run', last_exit_code:'Last result', actions:'Actions', on:'ENABLED', off:'DISABLED', success:'Success', failed:'Failed', never:'Not run', run:'Run now', enable:'Enable task', disable:'Disable task', edit:'Edit', remove:'Delete', copy:'Copy command', copied:'Copied', empty:'No matching scheduled tasks', count:'Total: {total} · Enabled: {active}', selected:'{count} selected', clear:'Clear selection', chooseAll:'Select all visible tasks', scroll:'Scroll horizontally to view more columns and actions', error:'Task operation failed. Please try again' },
  es: { name:'Tarea', enabled:'Estado', schedule:'Cron', command:'Comando', last_run:'Última ejecución', last_exit_code:'Resultado', actions:'Acciones', on:'ACTIVA', off:'INACTIVA', success:'Correcto', failed:'Error', never:'Sin ejecutar', run:'Ejecutar', enable:'Activar', disable:'Desactivar', edit:'Editar', remove:'Eliminar', copy:'Copiar comando', copied:'Copiado', empty:'No hay tareas coincidentes', count:'Total: {total} · Activas: {active}', selected:'{count} seleccionadas', clear:'Limpiar', chooseAll:'Seleccionar todas las tareas visibles', scroll:'Desplácese horizontalmente para ver más columnas', error:'No se pudo completar la operación' },
  ja: { name:'タスク名', enabled:'状態', schedule:'Cron式', command:'実行コマンド', last_run:'最終実行', last_exit_code:'結果', actions:'操作', on:'有効', off:'無効', success:'成功', failed:'失敗', never:'未実行', run:'今すぐ実行', enable:'有効にする', disable:'無効にする', edit:'編集', remove:'削除', copy:'コマンドコピー', copied:'コピー済み', empty:'一致するタスクがありません', count:'合計 {total} 件 · 有効 {active} 件', selected:'{count} 件選択中', clear:'選択解除', chooseAll:'表示中のタスクを選択', scroll:'横にスクロールして他の列を表示', error:'タスク操作が失敗しました' },
  pt: { name:'Tarefa', enabled:'Estado', schedule:'Cron', command:'Comando', last_run:'Última execução', last_exit_code:'Resultado', actions:'Ações', on:'ATIVA', off:'INATIVA', success:'Sucesso', failed:'Falha', never:'Não executada', run:'Executar agora', enable:'Ativar', disable:'Desativar', edit:'Editar', remove:'Excluir', copy:'Copiar comando', copied:'Copiado', empty:'Nenhuma tarefa encontrada', count:'Total: {total} · Ativas: {active}', selected:'{count} selecionadas', clear:'Limpar seleção', chooseAll:'Selecionar tarefas visíveis', scroll:'Role horizontalmente para ver outras colunas', error:'Falha na operação da tarefa' },
};

function taskText(key, vars = {}) {
  const lang = typeof getLang === 'function' ? getLang() : 'zh';
  let message = (TASK_TEXT[lang] || TASK_TEXT.en)[key] || TASK_TEXT.en[key] || key;
  for (const [name, value] of Object.entries(vars)) {
    message = message.replace('{' + name + '}', String(value));
  }
  return message;
}

function taskEscape(value) {
  return String(value ?? '').replace(/[&<>"']/g, ch => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  }[ch]));
}

function taskCommand(task) {
  return [String(task.cmd || ''), ...(Array.isArray(task.args) ? task.args.map(String) : [])].filter(Boolean).join(' ');
}

function taskIcon(name) {
  const value = String(name || '').toLowerCase();
  if (value.includes('mail')) return 'mail';
  if (value.includes('report')) return 'description';
  if (value.includes('data') || value.includes('backup')) return 'database';
  if (value.includes('video')) return 'videocam';
  return 'calendar_month';
}

function formatTaskTime(time) {
  if (!time) return '—';
  const date = new Date(time);
  if (!Number.isFinite(date.getTime())) return '—';
  const lang = typeof getLang === 'function' ? getLang() : 'zh';
  const locale = {zh:'zh-CN', en:'en-US', es:'es-ES', ja:'ja-JP', pt:'pt-BR'}[lang] || 'zh-CN';
  return new Intl.DateTimeFormat(locale, {month:'2-digit', day:'2-digit',hour:'2-digit',minute:'2-digit',hour12:false}).format(date);
}

function taskAction(task, action, icon, title, extra = '') {
  return `<button type="button" class="pf-action ${extra}" data-task-action="${action}" data-name="${taskEscape(task.name)}" aria-label="${taskEscape(title + ': ' + task.name)}" title="${taskEscape(title)}"${action === 'toggle' ? ' aria-pressed="' + String(Boolean(task.enabled)) + '"' : ''}><span class="material-symbols-outlined" aria-hidden="true">${icon}</span></button>`;
}

function renderTaskRow(task) {
  const on = Boolean(task.enabled);
  const name = taskEscape(task.name);
  const command = taskCommand(task);
  const checked = selectedTasks.has(task.name);
  const lastTime = formatTaskTime(task.last_run);
  const lastFull = task.last_run ? new Date(task.last_run).toLocaleString() : taskText('never');
  const resultKey = ['running','cancelled','interrupted','timeout'].includes(task.status) ? task.status : task.last_exit_code == null ? 'never' : Number(task.last_exit_code) === 0 ? 'success' : 'failed';
  const resultIcon = {never:'remove', success:'check_circle', failed:'error', running:'progress_activity',timeout:'timer_off',cancelled:'block',interrupted:'warning'}[resultKey];
  return `
<tr data-task-name="${name}" class="${checked ? 'pf-is-selected' : ''}">
  <td><input type="checkbox" class="pf-task-select" data-name="${name}" aria-label="${name}" ${checked ? 'checked' : ''}></td>
  <td>
    <div class="pf-process-identity">
      <span class="pf-process-icon ${on ? '' : 'pf-process-icon--stopped'}" aria-hidden="true"><span class="material-symbols-outlined">${taskIcon(task.name)}</span></span>
      <span class="pf-process-name" title="${name}">${name}</span>
    </div>
  </td>
  <td><span class="pf-status pf-task-status--${on ? 'enabled' : 'disabled'}">${taskText(on ? 'on' : 'off')}</span></td>
  <td><span class="pf-task-cron" title="${taskEscape(task.schedule)}">${taskEscape(task.schedule)}</span></td>
  <td><div class="pf-command" title="${taskEscape(command)}">
    <span class="pf-command-dollar" aria-hidden="true">$</span>
    <span class="pf-command-text">${taskEscape(command)}</span>
    ${taskAction(task, 'copy', 'content_copy', taskText('copy'))}
  </div></td>
  <td class="pf-task-last-run" title="${taskEscape(lastFull)}">${taskEscape(lastTime)}</td>
  <td><span class="pf-task-result pf-task-result--${resultKey}" title="${taskEscape(resultKey === 'running' ? '运行中' : resultKey === 'cancelled' ? '已取消' : resultKey === 'timeout' ? '已超时' : resultKey === 'interrupted' ? '已中断' : task.last_exit_code == null ? taskText('never') : 'Exit code: ' + task.last_exit_code)}"><span class="material-symbols-outlined" aria-hidden="true">${resultIcon}</span>${({running:'运行中',timeout:'超时',cancelled:'已取消',interrupted:'已中断'})[resultKey] || taskText(resultKey)}</span></td>
  <td><div class="pf-actions">
    ${taskAction(task, 'run', 'play_arrow', taskText('run'), 'pf-task-action-run')}
    ${taskAction(task, 'history', 'history', '查看历史')}
    ${task.status === 'running' ? taskAction(task, 'stop', 'stop_circle', '停止运行', 'pf-action--delete') : ''}
    ${taskAction(task, 'toggle', on ? 'toggle_on' : 'toggle_off', taskText(on ? 'disable' : 'enable'), 'pf-task-action-toggle')}
    ${taskAction(task, 'edit', 'edit', taskText('edit'))}
    ${taskAction(task, 'delete', 'delete', task.project ? '移除定时计划（保留项目脚本）' : taskText('remove'), 'pf-action--delete')}
  </div></td>
</tr>`;
}

let cachedTasks = [];
let editingName = null;
let taskSearch = '';
let taskSortKey = null;
let taskSortAsc = true;
let taskRequestSequence = 0;
const selectedTasks = new Set();
const runningTaskRequests = new Set();
let activeHistoryLogRequest=0;

function visibleTasks() {
  const needle = taskSearch.trim().toLocaleLowerCase();
  const rows = cachedTasks.filter(task => task.schedule && (
    !needle || [task.name, task.schedule, taskCommand(task),
      task.enabled ? taskText('on') : taskText('off'),
      task.last_exit_code == null ? taskText('never') : Number(task.last_exit_code) === 0 ? taskText('success') : taskText('failed')
    ].some(s => String(s || '').toLocaleLowerCase().includes(needle))
  ));
  if (taskSortKey) {
    rows.sort((a, b) => {
      const value = t => {
        if (taskSortKey === 'last_run') return t.last_run ? new Date(t.last_run).getTime() || 0 : -1;
        if (taskSortKey === 'last_exit_code') return t.last_exit_code == null ? -1 : Number(t.last_exit_code);
        if (taskSortKey === 'enabled') return Number(Boolean(t.enabled));
        return String(t[taskSortKey] || '');
      };
      const left = value(a), right = value(b);
      const comparison = typeof left === 'string' ? left.localeCompare(right) : left - right;
      return (taskSortAsc ? 1 : -1) * comparison;
    });
  }
  return rows;
}

function renderTaskTable() {
  const body = document.getElementById('task-rows');
  if (!body) return;
  const rows = visibleTasks();
  body.innerHTML = rows.length ? rows.map(renderTaskRow).join('') :
    `<tr><td class="pf-process-empty" colspan="8">${taskEscape(taskText('empty'))}</td></tr>`;

  document.querySelectorAll('[data-task-label]').forEach(el => {
    el.textContent = taskText(el.dataset.taskLabel);
  });
  document.querySelectorAll('[data-task-sort]').forEach(el => {
    const key = el.dataset.taskSort;
    const th = el.closest('th');
    if (th) th.setAttribute('aria-sort', key === taskSortKey ? (taskSortAsc ? 'ascending' : 'descending') : 'none');
    const indicator = el.querySelector('.pf-sort-icon');
    if (indicator) indicator.textContent = key === taskSortKey ? (taskSortAsc ? 'arrow_upward' : 'arrow_downward') : 'unfold_more';
  });
  const count = document.getElementById('task-count');
  if (count) count.textContent = taskText('count', {
    total: cachedTasks.length, active: cachedTasks.filter(task => task.enabled).length,
  });

  const all = document.getElementById('task-select-all');
  if (all) {
    const selectedVisible = rows.filter(task => selectedTasks.has(task.name)).length;
    all.checked = rows.length > 0 && selectedVisible === rows.length;
    all.indeterminate = selectedVisible > 0 && selectedVisible < rows.length;
    all.setAttribute('aria-label', taskText('chooseAll'));
  }
  const toolbar = document.getElementById('task-selection-toolbar');
  if (toolbar) toolbar.hidden = selectedTasks.size === 0;
  const label = document.getElementById('task-selected-label');
  if (label) label.textContent = taskText('selected', {count: selectedTasks.size});
  const clear = document.getElementById('task-clear-selection');
  if (clear) clear.textContent = taskText('clear');
  const hint = document.getElementById('task-scroll-hint');
  if (hint) hint.textContent = taskText('scroll');
}

function taskError(error) {
  console.error('PerchFlow task:', error);
  const el = document.getElementById('task-feedback');
  if (el) {
    el.textContent = taskText('error') + ': ' + (error.message || error);
    el.hidden = false;
    el.style.display = 'block';
    el.style.color = '#f8a0ad';
  }
}

async function loadTasks() {
  const sequence = ++taskRequestSequence;
  try {
    const result = await apiJson('/api/tasks?scope=scheduled');
    if (sequence !== taskRequestSequence) return;
    cachedTasks = Array.isArray(result.tasks) ? result.tasks : [];
    const names = new Set(cachedTasks.map(task => task.name));
    for (const name of selectedTasks) if (!names.has(name)) selectedTasks.delete(name);
    renderTaskTable();
  } catch (error) {
    taskError(error);
    if (!cachedTasks.length) {
      const body = document.getElementById('task-rows');
      if (body) body.innerHTML = `<tr><td class="pf-process-empty" colspan="8">${taskEscape(taskText('error'))}</td></tr>`;
    }
  }
}

async function executeTaskOperation(path, options) {
  const response = await api(path, options);
  const result = await response.json();
  if (!response.ok || !result.ok) throw new Error(result.error || 'Request failed');
  await loadTasks();
  return result;
}

async function runNow(name) {
  if(runningTaskRequests.has(name))return;
  runningTaskRequests.add(name);
  try {await executeTaskOperation('/api/tasks/'+name+'/run',{method:'POST'});}
  catch(error){taskError(error);}
  finally{runningTaskRequests.delete(name);}
}

function historyTimestamp(time){
  if(!time)return '—';
  const d=new Date(time);
  if(!Number.isFinite(d.valueOf()))return '—';
  return d.toLocaleString(typeof getLang==='function'&&getLang()==='en'?'en-US':'zh-CN',
    {year:'numeric',month:'2-digit',day:'2-digit',
     hour:'2-digit',minute:'2-digit',second:'2-digit',hour12:false});
}
function historyStatus(status){
  const map={success:'成功',failed:'失败',running:'运行中',cancelled:'已取消',
    blocked:'预检阻止',timeout:'超时',interrupted:'中断'};
  return map[status]||status||'—';
}
async function showTaskHistory(name){
  try{
    const data=await apiJson('/api/tasks/'+encodeURIComponent(name)+'/history');
    if(!data.ok)throw Error(data.error||'History unavailable');
    const modal=document.getElementById('task-history-modal');
    const list=document.getElementById('task-history-list');
    const log=document.getElementById('task-run-log');
    document.getElementById('task-history-title').textContent=name+' · 执行历史';
    list.replaceChildren();
    log.textContent='选择上方一条记录查看对应的日志';
    const runs=Array.isArray(data.runs)?data.runs.slice(0,30):[];
    if(!runs.length)list.textContent='暂无执行记录';
    for(const run of runs){
      const b=document.createElement('button');
      b.type='button';b.className='pf-task-history-record';
      b.dataset.runId=run.run_id;
      const heading=document.createElement('span');
      heading.className='pf-task-history-record-top';
      const time=document.createElement('strong');
      time.textContent=historyTimestamp(run.started_at);
      const state=document.createElement('span');
      state.className='pf-task-history-state';
      state.dataset.status=run.status||'none';
      state.textContent=historyStatus(run.status);
      heading.append(time,state);
      const detail=document.createElement('small');
      detail.textContent='退出码：'+(run.exit_code??'—')+'  ·  时长：'+
        (run.duration_ms!=null?(Number(run.duration_ms)/1000).toFixed(1)+'s':'—');
      b.append(heading,detail);
      b.addEventListener('click',()=>{
        list.querySelectorAll('.pf-task-history-record').forEach(row=>{
          row.setAttribute('aria-current',String(row===b));
        });
        void showTaskLog(name,run.run_id);
      });
      list.append(b);
    }
    modal.classList.remove('hidden');modal.classList.add('flex');
    if(runs.length)list.querySelector('.pf-task-history-record').click();
  }catch(error){taskError(error);}
}

async function showTaskLog(name,runId){
  const request=++activeHistoryLogRequest;
  const log=document.getElementById('task-run-log');
  log.textContent='正在读取日志…';
  try{
    const data=await apiJson('/api/tasks/'+encodeURIComponent(name)+'/log/'+encodeURIComponent(runId));
    if(request!==activeHistoryLogRequest)return;
    if(!data.ok)throw Error(data.error||'No log');
    log.textContent=(data.truncated?'[日志过长，仅显示末尾内容]\n':'')+(data.content||'（没有输出）');
    log.scrollTop=0;
  }catch(error){
    if(request===activeHistoryLogRequest)log.textContent=error.message||String(error);
  }
}

function closeTaskHistory() {
  ++activeHistoryLogRequest;
  const modal = document.getElementById('task-history-modal');
  modal.classList.add('hidden');
  modal.classList.remove('flex');
}

async function stopTask(name) {
  if (!await PerchDialog.confirm('确认停止正在运行的任务？',{title:'停止任务',danger:true})) return;
  try { await executeTaskOperation(`/api/tasks/${encodeURIComponent(name)}/stop`, {method:'POST'}); }
  catch (error) { taskError(error); }
}

async function deleteTask(name) {
  const task=cachedTasks.find(item=>item.name===decodeURIComponent(name));
  if(!task)return;
  const projectLinked=Boolean(task.project);
  const label=projectLinked
    ? '仅移除「'+task.name+'」的 Cron 计划，保留项目脚本、文件与执行历史。'
    : t('confirm.delete_task');
  if(!await PerchDialog.confirm(label,{
      title:projectLinked?'移除定时计划':'删除定时任务',danger:true}))return;
  try{
    await executeTaskOperation('/api/tasks/'+encodeURIComponent(task.name)+
      (projectLinked?'/schedule':''),{method:'DELETE'});
  }catch(error){taskError(error);}
}

async function toggleEnabled(name, enabled) {
  const existing=cachedTasks.find(item=>item.name===decodeURIComponent(name));
  if(!existing||!existing.schedule)return;
  try{
    await executeTaskOperation('/api/tasks/'+encodeURIComponent(existing.name)+'/enabled',
      {method:'PUT',body:JSON.stringify({enabled})});
  }catch(error){taskError(error);}
}

async function copyTaskCommand(name, button) {
  const item = cachedTasks.find(task => task.name === name);
  if (!item) return;
  const text = taskCommand(item);
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const field = document.createElement('textarea');
      field.value = text;
      field.style.position = 'fixed';
      field.style.opacity = '0';
      document.body.appendChild(field);
      field.select();
      const copied = document.execCommand('copy');
      field.remove();
      if (!copied) throw new Error('Clipboard unavailable');
    }
    button.title = taskText('copied');
    setTimeout(() => { if (button.isConnected) button.title = taskText('copy'); }, 1250);
  } catch (error) { taskError(error); }
}

function parseTaskArgs(raw) {
  const result = [];
  let current = '', quote = null, active = false;
  for (let i = 0; i < raw.length; i++) {
    const ch = raw[i];
    if (ch === '\\' && quote === '"' && (raw[i+1] === '\\' || raw[i+1] === '"')) {
      current += raw[++i];
      active = true;
    } else if (ch === '"' || ch === "'") {
      if (quote === ch) { quote = null; } else if (!quote) { quote = ch; } else { current += ch; }
      active = true;
    } else if (/\s/.test(ch) && !quote) {
      if (active) { result.push(current); current = ''; active = false; }
    } else { current += ch; active = true; }
  }
  if (quote) throw new Error('参数引号没有闭合');
  if (active) result.push(current);
  return result;
}

function openTaskModal() {
  editingName = null;
  document.getElementById('task-modal-title').textContent = t('modal.add_task');
  document.getElementById('task-name').value = '';
  document.getElementById('task-name').disabled = false;
  document.getElementById('task-cmd').value = '';
  document.getElementById('task-args').value = '';
  document.getElementById('task-cwd').value = '';
  document.getElementById('task-timeout').value = '3600';
  document.getElementById('task-schedule').value = '';
  document.getElementById('task-enabled').checked = true;
  document.getElementById('task-err').classList.add('hidden');
  showTaskModal();
}

function editTask(name) {
  // Do not shadow the global t() translation function with a local task.
  const task = cachedTasks.find(item => item.name === decodeURIComponent(name));
  if (!task) return;
  editingName = task.name;
  document.getElementById('task-modal-title').textContent = t('modal.edit_task');
  document.getElementById('task-name').value = task.name;
  document.getElementById('task-name').disabled = false;
  document.getElementById('task-cmd').value = task.cmd;
  document.getElementById('task-args').value = (task.args || []).map(v => /\s|"/.test(v) ? JSON.stringify(v) : v).join(' ');
  document.getElementById('task-cwd').value = task.cwd || '';
  document.getElementById('task-timeout').value = task.timeout_secs ?? 0;
  document.getElementById('task-schedule').value = task.schedule;
  document.getElementById('task-enabled').checked = task.enabled;
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
  let args;
  try { args = parseTaskArgs(document.getElementById('task-args').value); }
  catch (error) { const err = document.getElementById('task-err'); err.textContent = error.message; err.classList.remove('hidden'); return; }
  const schedule = document.getElementById('task-schedule').value.trim();
  const enabled = document.getElementById('task-enabled').checked;
  const errEl = document.getElementById('task-err');
  if (!name || !cmd || !schedule) {
    errEl.textContent = t('error.name_cmd_cron_required');
    errEl.classList.remove('hidden');
    return;
  }
  const cwd = document.getElementById('task-cwd').value.trim() || null;
  const timeout_secs = Number(document.getElementById('task-timeout').value);
  if (!Number.isSafeInteger(timeout_secs) || timeout_secs < 0) { errEl.textContent = '超时秒数必须是非负整数'; errEl.classList.remove('hidden'); return; }
  const body = {name,cmd,args,cwd,schedule,enabled,timeout_secs};
  if (cwd) {
    try {
      const checked = await api('/api/processes/validate-cwd', {
        method:'POST', body:JSON.stringify({cwd}),
      });
      const validation = await checked.json();
      if (!validation.ok) {
        const messages = {
          cwd_not_absolute:'工作目录必须是绝对路径',
          cwd_not_found:'工作目录不存在',
          cwd_not_directory:'所选路径不是文件夹',
          cwd_inaccessible:'无法访问工作目录',
          cwd_too_long:'工作目录路径过长',
        };
        errEl.textContent = messages[validation.error] || validation.error || '工作目录验证失败';
        errEl.classList.remove('hidden');
        return;
      }
    } catch (error) {
      errEl.textContent = error.message || String(error);
      errEl.classList.remove('hidden');
      return;
    }
  }
  const path = editingName ? `/api/tasks/${encodeURIComponent(editingName)}` : '/api/tasks';
  const method = editingName ? 'PUT' : 'POST';
  try {
    const response = await api(path, {method,body:JSON.stringify(body)});
    const result = await response.json();
    if (!response.ok || !result.ok) {
      errEl.textContent = result.error || t('error.save_failed');
      errEl.classList.remove('hidden');
      return;
    }
    closeTaskModal();
    await loadTasks();
  } catch (error) {
    errEl.textContent = String(error.message || error);
    errEl.classList.remove('hidden');
  }
}

async function bindExistingScript() {
  try{
    const result=await apiJson('/api/tasks?scope=unscheduled');
    if(result.ok===false)throw Error(result.error||'获取已有脚本失败');
    const options=(result.tasks||[]).filter(task=>Boolean(task.project));
    if(!options.length){
      await PerchDialog.notice('没有可关联的项目脚本。请先在「项目」页面添加 Python 脚本。');
      return;
    }
    const selected=await PerchDialog.chooseScript(options);
    if(!selected)return;
    await PerchDialog.cron({
      title:'为「'+selected.name+'」添加定时计划',
      editing:false,initial:'0 0 0 * * *',
      onSave:async schedule=>{
        await executeTaskOperation('/api/tasks/'+encodeURIComponent(selected.name)+'/schedule',
          {method:'PUT',body:JSON.stringify({schedule,enabled:true})});
      }
    });
  }catch(error){taskError(error);}
}

document.addEventListener('DOMContentLoaded', () => {
  document.getElementById('task-bind-existing')?.addEventListener('click',()=>{void bindExistingScript();});
  const input = document.getElementById('task-search');
  if (input) input.addEventListener('input', () => {
    taskSearch = input.value;
    renderTaskTable();
  });

  const table = document.querySelector('.pf-task-table');
  if (table) {
    table.addEventListener('click', event => {
      const sort = event.target.closest('[data-task-sort]');
      if (sort) {
        const key = sort.dataset.taskSort;
        taskSortAsc = key === taskSortKey ? !taskSortAsc : true;
        taskSortKey = key;
        renderTaskTable();
        return;
      }
      const button = event.target.closest('button[data-task-action]');
      if (!button || button.disabled) return;
      const name = button.dataset.name;
      const encoded = encodeURIComponent(name);
      const action = button.dataset.taskAction;
      if (action === 'run') void runNow(encoded);
      else if (action === 'toggle') void toggleEnabled(encoded, button.getAttribute('aria-pressed') !== 'true');
      else if (action === 'history') void showTaskHistory(name);
      else if (action === 'stop') void stopTask(name);
      else if (action === 'edit') editTask(encoded);
      else if (action === 'delete') void deleteTask(encoded);
      else if (action === 'copy') void copyTaskCommand(name, button);
    });
    table.addEventListener('change', event => {
      if (event.target.id === 'task-select-all') {
        const checked = event.target.checked;
        for (const task of visibleTasks()) {
          if (checked) selectedTasks.add(task.name);
          else selectedTasks.delete(task.name);
        }
        renderTaskTable();
        return;
      }
      if (event.target.matches('.pf-task-select')) {
        const name = event.target.dataset.name;
        if (event.target.checked) selectedTasks.add(name);
        else selectedTasks.delete(name);
        renderTaskTable();
      }
    });
  }

  const clear = document.getElementById('task-clear-selection');
  if (clear) clear.addEventListener('click', () => {
    selectedTasks.clear();
    renderTaskTable();
  });
  document.querySelectorAll('[data-lang-select]').forEach(select => {
    select.addEventListener('change', renderTaskTable);
  });
  void loadTasks();
  setInterval(loadTasks, 3000);
});
