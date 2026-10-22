/* PerchFlow project script detail: one run_id per log, visible-page polling. */
'use strict';
const query = new URLSearchParams(location.search);
const projectName = query.get('project') || '';
const taskName = query.get('task') || '';
const en = {
  '空闲':'Idle','运行中':'Running','成功':'Success','失败':'Failed','已取消':'Cancelled',
  '超时':'Timed out','预检阻止':'Preflight blocked','意外中断':'Interrupted',
  '当前未运行':'Not running','启用':'Enabled','停用':'Disabled',
  '手动执行':'Manual run','手动 + 定时':'Manual + scheduled',
  '定时已启用':'Schedule active','Cron 已暂停':'Cron paused','未设置定时':'No schedule',
  '尚未执行':'Not run','查看更多 →':'View more →','收起':'Collapse',
  '等待查看日志…':'Waiting for log output…',
  '暂无执行日志':'No run logs yet',
  '等待脚本输出…':'Waiting for script output…',
  '日志不存在、尚未生成，或已按三次保留规则清理。':'Log missing, not created, or removed under 3-run retention.',
  '正在加载…':'Loading…','没有执行记录':'No run history',
  '已保留日志':'Log retained','旧日志可能已清理':'Older log may be expired',
  '未发现可识别的文件，动态计算的路径需要人工检查。':'No recognizable files; dynamic paths may need manual inspection.',
  '代码候选':'Code candidate','项目输入':'Project input','项目输出':'Project output',
  '存在':'Available','未找到':'Missing','打开文件':'Open file',
  '任务已经结束，状态已同步。':'Task already finished; status synchronized.',
  '点击确定会在 PerchFlow 所在电脑上执行真实 Python 脚本。继续？':'This will execute the real Python script on the PerchFlow computer. Continue?',
  '停止当前运行的 Python 脚本？':'Stop the running Python script?',
  '检测到的路径只是代码静态候选，并非确定的实际文件依赖。':'Static code candidate; not a verified runtime dependency.',
};
function tx(value) { return typeof getLang === 'function' && getLang() === 'en' ? (en[value] || value) : value; }
function node(id) { return document.getElementById(id); }
function make(tag,cls,value) { const n=document.createElement(tag);if(cls)n.className=cls;if(value!==undefined)n.textContent=String(value);return n; }
function localizeStatic() {
  document.querySelectorAll('[data-pf-en]').forEach(el=>{
    if(!el.hasAttribute('data-pf-zh'))el.setAttribute('data-pf-zh',el.textContent);
    el.textContent=getLang()==='en'?el.dataset.pfEn:el.dataset.pfZh;
  });
  document.title=getLang()==='en'?'PerchFlow · Script details':'PerchFlow · 脚本详情';
}
async function get(path,options) {
  const response=await api(path,options||{});
  const payload=await response.json();
  if(!response.ok||payload.ok===false)throw Error(payload.error||'HTTP '+response.status);
  return payload;
}
const apiTask='/api/tasks/'+encodeURIComponent(taskName);
const apiFiles='/api/projects/'+encodeURIComponent(projectName)+'/tasks/'+encodeURIComponent(taskName);
let latestTask=null;
let historyRuns=[];
let selectedId=null;
let followLatest=true;
let logOpted=false;
let shownLog='';
let lastLogId=null;
let polling=false;
let fileLoading=false;
let checkedOwnership=false;
let actionBusy=false;
let activeLogRequest=0;
let settingsSaving=false;
let projectRoot='';

function message(text) {
  const label=node('task-message');
  label.textContent=tx(text);label.hidden=false;
}
function duration(sec) {
  const seconds=Math.max(0,Number(sec)||0);
  const h=Math.floor(seconds/3600),m=Math.floor((seconds%3600)/60),s=Math.floor(seconds%60);
  return String(h).padStart(2,'0')+':'+String(m).padStart(2,'0')+':'+String(s).padStart(2,'0');
}
function renderTask(task) {
  latestTask=task;
  const script=(task.args||[]).find(a=>String(a).toLowerCase().endsWith('.py')) || '';
  const scriptName=String(script).replace(/\\/g,'/').split('/').pop() || '—';
  node('task-name').textContent=task.name;
  node('task-breadcrumb-name').textContent=task.name;
  node('task-log-script-name').textContent=scriptName;
  node('task-path').textContent=scriptName;
  node('task-path').title=scriptName;
  node('task-project-name').textContent=projectName;
  node('task-project-name').href='/projects?project='+encodeURIComponent(projectName);
  node('task-back').href='/projects?project='+encodeURIComponent(projectName);
  const active=task.status==='running';
  node('task-status').textContent=tx(active?'运行中':'空闲');
  node('task-pid').textContent=active && task.pid!=null ? task.pid : '—';
  if(active) node('task-uptime').textContent=duration(task.uptime_secs);
  node('task-schedule').textContent=tx(task.schedule?'手动 + 定时':'手动执行');
  node('task-schedule-hint').textContent=task.schedule?(task.enabled?tx('定时已启用'):tx('Cron 已暂停')):tx('未设置定时');
  node('task-python').textContent=task.cmd||'—';
  node('task-cwd').textContent=task.cwd||'—';
  node('task-args').textContent=(task.args||[]).slice(1).join(' ')||'—';
  for(const key of ['task-python','task-cwd','task-args']) {
    node(key).title=node(key).textContent;
  }
  node('task-schedule-remove').hidden=!task.schedule;
  node('task-settings').disabled=active||settingsSaving;
  node('task-run').disabled=active||actionBusy;
  node('task-stop').disabled=!active||actionBusy;
}function logLineCount(value) {
  return value + ' ' + (getLang() === 'en' ? 'lines' : '\u884c');
}
function updateLogLiveButton() {
  const enabled = node('task-log-auto').checked;
  const toggle = node('task-log-live');
  toggle.classList.toggle('is-on', enabled);
  toggle.setAttribute('aria-checked',String(enabled));
  const mode = node('task-log-view-mode');
  mode.dataset.live=String(enabled);
  mode.textContent=enabled
    ? (getLang()==='en' ? 'Live mode' : '\u5b9e\u65f6\u6a21\u5f0f')
    : (getLang()==='en' ? 'Paused' : '\u5df2\u6682\u505c');
}
function logReset() {
  activeLogRequest++;
  shownLog='';
  lastLogId=null;
  const view=node('task-log-content');
  view.replaceChildren();
  view.dataset.empty=tx('暂无执行日志');
  view.scrollTop=0;
  node('task-log-count').textContent=logLineCount(0);
}
function renderScriptLog(raw) {
  // Uses the Process Detail terminal's pf-log-line and color classes,
  // but never inserts untrusted script output as HTML.
  const view=node('task-log-content');
  const nearEnd=view.scrollHeight-view.scrollTop-view.clientHeight<60;
  const oldScroll=view.scrollTop;
  const all=raw ? raw.split(/\r?\n/) : [];
  if(all.length && all[all.length-1]==='')all.pop();
  const limit=Number(node('task-log-lines').value)||500;
  const lines=all.slice(-limit);
  const fragment=document.createDocumentFragment();
  for (const line of lines) {
    const item=document.createElement('div');
    item.className='pf-log-line';
    if(/\b(?:ERROR|FATAL|CRITICAL)\b|\[ERR\]/i.test(line))item.classList.add('pf-log-line--error');
    else if(/\b(?:WARN|WARNING)\b/i.test(line))item.classList.add('pf-log-line--warn');
    else if(/\bINFO\b/i.test(line))item.classList.add('pf-log-line--info');
    item.textContent=line;
    fragment.appendChild(item);
  }
  view.replaceChildren(fragment);
  view.dataset.empty=tx('\u7b49\u5f85\u811a\u672c\u8f93\u51fa\u2026');
  node('task-log-count').textContent=logLineCount(lines.length);
  view.scrollTop=nearEnd?view.scrollHeight:oldScroll;
}
let showAllHistory=false;
function runStateLabel(status) {
  return tx(({running:'运行中',success:'成功',failed:'失败',cancelled:'已取消',timeout:'超时',
    blocked:'预检阻止',interrupted:'意外中断'})[status]||status||'未知');
}
function renderHistory(runs) {
  historyRuns=runs;
  const list=node('task-history');
  list.replaceChildren();
  if(!runs.length){
    list.append(make('p','pf-taskdetail-notice',tx('没有执行记录')));
    node('task-last-result').textContent=tx('尚未执行');
    node('task-last-time').textContent='—';
    if(latestTask?.status!=='running')node('task-uptime').textContent='—';
    selectedId=null;
    logOpted=false;
    logReset();
    return;
  }
  const latest=runs[0];
  const result=node('task-last-result');
  result.textContent=runStateLabel(latest.status);
  result.dataset.tone=latest.status==='success'?'success':
    (['failed','cancelled','blocked','timeout','interrupted'].includes(latest.status)?'error':'none');
  node('task-last-time').textContent=latest.started_at?
    new Date(latest.started_at).toLocaleString():'—';
  if(latestTask?.status!=='running')node('task-uptime').textContent=
    latest.duration_ms==null?'—':duration(latest.duration_ms/1000);
  node('task-history-more').hidden=runs.length<=3;
  node('task-history-more').textContent=tx(showAllHistory?'收起':'查看更多 →');
  for(const run of runs.slice(0,showAllHistory?12:3)) {
    const item=make('button','pf-taskdetail-history-item');
    item.type='button';
    item.setAttribute('aria-current',String(run.run_id===selectedId));
    const when=make('span','',run.started_at?
      new Date(run.started_at).toLocaleString():'—');
    const state=make('span','pf-taskdetail-history-state',runStateLabel(run.status));
    state.dataset.tone=run.status==='success'?'success':
      (run.status==='running'?'none':'error');
    const timing=make('small','',run.duration_ms==null?'—':duration(run.duration_ms/1000));
    item.append(when,state,timing);
    item.title=tx(run.log_available?'已保留日志':'旧日志可能已清理');
    item.addEventListener('click',()=>{
      followLatest=false;
      if(selectedId!==run.run_id) {
        selectedId=run.run_id;
        logReset();
      }
      logOpted=true;
      renderHistory(historyRuns);
      void readLog();
    });
    list.append(item);
  }
}function pickLatest(runs) {
  if(!runs.length)return;
  const retained=runs.some(item=>item.run_id===selectedId);
  const wanted=followLatest||!retained?runs[0].run_id:selectedId;
  if(wanted!==selectedId) {
    selectedId=wanted;
    // Every execution gets an independent log view; never show the prior run.
    logReset();
    logOpted=true;
  }
}
async function readLog() {
  if(!logOpted || !selectedId)return;
  const forId=selectedId;
  const request=++activeLogRequest;
  try{
    const data=await get(apiTask+'/log/'+encodeURIComponent(forId));
    if(request!==activeLogRequest || selectedId!==forId)return;
    const raw=String(data.content||'');
    if(raw!==shownLog || lastLogId!==forId){
      shownLog=raw;
      lastLogId=forId;
      renderScriptLog(raw);
    }
  }catch(_){
    if(request===activeLogRequest && selectedId===forId){
      const view=node('task-log-content');
      view.replaceChildren();
      view.dataset.empty=tx('\u65e5\u5fd7\u4e0d\u5b58\u5728\u3001\u5c1a\u672a\u751f\u6210\uff0c\u6216\u5df2\u6309\u4e09\u6b21\u4fdd\u7559\u89c4\u5219\u6e05\u7406\u3002');
      node('task-log-count').textContent=logLineCount(0);
    }
  }
}
async function poll() {
  if(polling||document.hidden||!taskName||!projectName)return;
  polling=true;
  try {
    if(!checkedOwnership){
      const detail=await get('/api/projects/'+encodeURIComponent(projectName));
      if(!detail.project.tasks.includes(taskName))throw Error('Task does not belong to project');
      projectRoot=String(detail.project.root||'');
      checkedOwnership=true;
    }
    const [task,history]=await Promise.all([get(apiTask),get(apiTask+'/history')]);
    renderTask(task);
    const runs=Array.isArray(history.runs)?history.runs:[];
    pickLatest(runs);
    renderHistory(runs);
    if(logOpted && selectedId &&
       (node('task-log-auto').checked || lastLogId!==selectedId)) await readLog();
  }catch(error){message(error.message);}
  finally{polling=false;}
}
async function act(action) {
  if(!checkedOwnership || actionBusy)return;
  if(action==='stop'&&!await PerchDialog.confirm('停止当前运行的 Python 脚本？',
      {title:'停止脚本',danger:true}))return;
  actionBusy=true;
  node('task-run').disabled=true;
  node('task-stop').disabled=true;
  try {
    const result=await get(apiTask+(action==='run'?'/run':'/stop'),{method:'POST'});
    if(result.already_finished) message('任务已经结束，状态已同步。');
    if(action==='run' && result.run_id) {
      followLatest=true;
      selectedId=result.run_id;
      logReset();
      logOpted=true;
    }
    await poll();
  }catch(error){message(error.message);}
  finally{actionBusy=false;await poll();}
}
// Presentation only: the verified absolute path is still sent unchanged to
// open-file. Never mislabel a file outside the project root as a relative file.
function linkedFileLabel(resolved, root) {
  const path=String(resolved||'').replace(/\//g,'\\');
  const base=String(root||'').replace(/\//g,'\\').replace(/\\+$/,'');
  if(!base)return path;
  const prefix=base+'\\';
  return path.toLowerCase().startsWith(prefix.toLowerCase())
    ? path.slice(prefix.length) : path;
}
async function loadFiles() {
  if(fileLoading)return;
  fileLoading=true;
  const list=node('task-linked-files');
  list.replaceChildren(make('p','pf-taskdetail-notice',tx('正在加载…')));
  try {
    const [result,projectInfo]=await Promise.all([
      get(apiFiles+'/files'),
      projectRoot ? Promise.resolve(null) : get('/api/projects/'+encodeURIComponent(projectName))
    ]);
    if(projectInfo?.project?.tasks?.includes(taskName)){
      projectRoot=String(projectInfo.project.root||'');
    }
    list.replaceChildren();
    const files=Array.isArray(result.files)?result.files:[];
    if(!files.length)list.append(make('p','pf-taskdetail-notice',
      tx('未发现可识别的文件，动态计算的路径需要人工检查。')));
    for(const file of files){
      const row=make('div','pf-taskdetail-file'),left=make('div','pf-taskdetail-file-info');
      const displayedPath=linkedFileLabel(file.resolved_path,projectRoot);
      const filename=make('div','pf-taskdetail-file-name',displayedPath);
      filename.title=displayedPath;
      left.append(filename);
      const origin={code_candidate:'代码候选',registered_input:'项目输入',
        registered_output:'项目输出'}[file.source]||file.source;
      left.append(make('div','pf-taskdetail-file-meta',tx(origin)+' · '+
        tx(file.exists?'存在':'未找到')+(file.size_bytes==null?'':' · '+file.size_bytes+' B')));
      if(file.inferred)left.append(make('small','pf-taskdetail-notice',
        tx('检测到的路径只是代码静态候选，并非确定的实际文件依赖。')));
      const open=make('button','pf-taskdetail-btn',tx('打开文件'));
      open.type='button';open.disabled=!file.exists;
      open.addEventListener('click',async()=>{
        try {
          // The file-open button is the user's explicit confirmation.
          // The backend still enforces loopback access and the discovered-file allowlist.
          await get(apiFiles+'/open-file',{method:'POST',
            body:JSON.stringify({path:file.resolved_path,confirmed:true})});
        }catch(err){message(err.message);}
      });
      row.append(left,open);list.append(row);
    }
  }catch(error){list.replaceChildren(make('p','pf-taskdetail-notice',error.message));}
  finally{fileLoading=false;}
}
function showScriptSettings() {
  if(!checkedOwnership || !latestTask || latestTask.status==='running')return;
  const task=latestTask;
  const script=(task.args||[]).find(value=>String(value).toLowerCase().endsWith('.py'))||'';
  node('task-settings-python').value=task.cmd||'';
  node('task-settings-script').value=script;
  node('task-settings-cwd').value=task.cwd||'';
  node('task-settings-args').value=(task.args||[]).slice(1).join('\n');
  node('task-settings-timeout').value=String(task.timeout_secs??0);
  node('task-settings-error').hidden=true;
  node('task-settings-dialog').showModal();
}
async function saveScriptSettings(event) {
  event.preventDefault();
  if(settingsSaving || !checkedOwnership || !latestTask)return;
  const dialog=node('task-settings-dialog');
  const timeout=Number(node('task-settings-timeout').value);
  if(!Number.isSafeInteger(timeout)||timeout<0){
    node('task-settings-error').textContent=tx('执行超时必须是非负整数。');
    node('task-settings-error').hidden=false;return;
  }
  const request={
    cmd:node('task-settings-python').value.trim(),
    script:node('task-settings-script').value.trim(),
    cwd:node('task-settings-cwd').value.trim(),
    args:node('task-settings-args').value.split(/\r?\n/).filter(x=>x.length>0),
    timeout_secs:timeout
  };
  settingsSaving=true;
  node('task-settings-save').disabled=true;
  try {
    await get(apiFiles+'/settings',{method:'PUT',body:JSON.stringify(request)});
    dialog.close();
    await poll();
    await loadFiles();
  }catch(error){
    node('task-settings-error').textContent=error.message;
    node('task-settings-error').hidden=false;
  }finally{
    settingsSaving=false;
    node('task-settings-save').disabled=false;
  }
}
document.addEventListener('DOMContentLoaded',()=>{
  localizeStatic();
  document.addEventListener('perchflow:language-changed',()=>{
    localizeStatic();
    if(latestTask)renderTask(latestTask);
    renderHistory(historyRuns);
    updateLogLiveButton();
    if(logOpted)renderScriptLog(shownLog);
  });
  node('task-settings').addEventListener('click',showScriptSettings);
  node('task-settings-form').addEventListener('submit',saveScriptSettings);
  node('task-settings-close').addEventListener('click',()=>node('task-settings-dialog').close());
  node('task-settings-cancel').addEventListener('click',()=>node('task-settings-dialog').close());
  node('task-settings-dialog').addEventListener('click',event=>{
    if(event.target===node('task-settings-dialog'))node('task-settings-dialog').close();
  });
  const more=node('task-more'), menu=node('task-more-menu');
  more.addEventListener('click',()=>{
    menu.hidden=!menu.hidden;
    more.setAttribute('aria-expanded',String(!menu.hidden));
  });
  document.addEventListener('click',event=>{
    if(!event.target.closest('.pf-taskdetail-menu-wrap')){
      menu.hidden=true;
      more.setAttribute('aria-expanded','false');
    }
  });
  node('task-refresh').addEventListener('click',()=>{
    menu.hidden=true;
    void poll();
  });
  node('task-show-files').addEventListener('click',()=>{
    menu.hidden=true;
    document.getElementById('task-files-panel').scrollIntoView({behavior:'smooth',block:'center'});
  });
  node('task-history-more').addEventListener('click',()=>{
    showAllHistory=!showAllHistory;
    renderHistory(historyRuns);
  });
  node('task-log-clear').addEventListener('click',()=>{
    // Clear the visible console only; never remove retained log files.
    logOpted=false;
    logReset();
  });
  node('task-schedule-add').addEventListener('click',async()=>{
    if(!checkedOwnership||!latestTask)return;
    const wasScheduled=Boolean(latestTask.schedule);
    await PerchDialog.cron({
      title:wasScheduled?'修改定时计划':'添加定时计划',
      initial:latestTask.schedule||'0 0 0 * * *', editing:wasScheduled,
      onSave:async schedule=>{
        await get(apiTask+'/schedule',{method:'PUT',
          body:JSON.stringify({schedule,enabled:wasScheduled?latestTask.enabled:true})});
        await poll();
      }
    });
  });
  node('task-schedule-remove').addEventListener('click',async()=>{
    if(!checkedOwnership||!latestTask||!latestTask.schedule)return;
    if(!await PerchDialog.confirm('仅移除该脚本的 Cron 计划，保留脚本及日志，运行中的任务不受影响。',
      {title:'移除定时计划',danger:true}))return;
    try{await get(apiTask+'/schedule',{method:'DELETE'});await poll();}
    catch(e){message(e.message);}
  });
  node('task-run').addEventListener('click',()=>{void act('run');});
  node('task-stop').addEventListener('click',()=>{void act('stop');});
  node('task-log-live').addEventListener('click',()=>{
    node('task-log-auto').checked=!node('task-log-auto').checked;
    updateLogLiveButton();
    if(logOpted && node('task-log-auto').checked)void readLog();
  });
  node('task-log-lines').addEventListener('change',()=>{
    if(logOpted && lastLogId===selectedId)renderScriptLog(shownLog);
  });
  updateLogLiveButton();
  node('task-files-refresh').addEventListener('click',()=>{void loadFiles();});
  if(!taskName || !projectName) {message('Missing task or project name');return;}
  void poll();
  void loadFiles();
  setInterval(()=>{void poll();},2000);
});
