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
  '等待查看日志…':'Click View log to read output…',
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
  '将在运行 PerchFlow 的 Windows 电脑上用默认程序打开该文件。确认？':'Open this document on the PerchFlow Windows computer using its associated application?',
  '检测到的路径只是代码静态候选，并非确定的实际文件依赖。':'Static code candidate; not a verified runtime dependency.',
  '仅支持本机 Windows 交互桌面；服务模式可能无法显示应用窗口。':'Available only in an interactive Windows session; service mode may not show windows.',
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
  const script=(task.args||[]).find(a=>String(a).toLowerCase().endsWith('.py')) || task.name;
  const name=String(script).replace(/\\/g,'/').split('/').pop();
  node('task-name').textContent=name;
  node('task-breadcrumb-name').textContent=name;
  node('task-path').textContent=String(script);
  node('task-project-name').textContent=projectName;
  node('task-project-name').href='/projects?project='+encodeURIComponent(projectName);
  node('task-back').href='/projects?project='+encodeURIComponent(projectName);
  const active=task.status==='running';
  node('task-status').textContent=tx(active?'运行中':'空闲');
  node('task-pid').textContent=active && task.pid!=null ? task.pid : '—';
  if(active) node('task-uptime').textContent=duration(task.uptime_secs);
  node('task-cpu').textContent=active?(Number(task.cpu_percent)||0).toFixed(1)+'%':'—';
  node('task-memory').textContent=active?(Number(task.mem_bytes||0)/1048576).toFixed(1)+' MB':'—';
  node('task-schedule').textContent=tx(task.enabled?'手动 + 定时':'手动执行');
  node('task-schedule-hint').textContent=task.enabled?tx('定时已启用') :
    (task.schedule?tx('Cron 已暂停'):tx('未设置定时'));
  node('task-python').textContent=task.cmd||'—';
  node('task-cwd').textContent=task.cwd||'—';
  node('task-args').textContent=(task.args||[]).slice(1).join(' ')||'—';
  node('task-exit').textContent=task.last_exit_code==null?'—':task.last_exit_code;
  node('task-run').disabled=active||actionBusy;
  node('task-stop').disabled=!active||actionBusy;
}function logReset() {
  activeLogRequest++;
  shownLog='';
  lastLogId=null;
  node('task-log-content').textContent=tx('等待查看日志…');
  node('task-log-count').textContent='0 '+(getLang()==='en'?'lines':'行');
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
      selectedId=run.run_id;
      logOpted=false;
      node('task-log-show').disabled=false;
      logReset();
      renderHistory(historyRuns);
      document.getElementById('task-log-panel').scrollIntoView({block:'nearest',behavior:'smooth'});
    });
    list.append(item);
  }
}function pickLatest(runs) {
  if(!runs.length)return;
  const wanted=followLatest||!selectedId?runs[0].run_id:selectedId;
  if(wanted!==selectedId) {
    selectedId=wanted;
    // A new run must ALWAYS start at an empty log view.
    logReset();
  }
}
async function readLog() {
  if(!logOpted||!selectedId)return;
  const forId=selectedId;
  const req=++activeLogRequest;
  try {
    const data=await get(apiTask+'/log/'+encodeURIComponent(forId));
    if(req!==activeLogRequest||selectedId!==forId)return;
    const next=String(data.content||'');
    if(next!==shownLog || lastLogId!==forId){
      const view=node('task-log-content');
      const atEnd=view.scrollHeight-view.scrollTop-view.clientHeight<60;
      const priorTop=view.scrollTop;
      view.textContent=next||tx('等待脚本输出…');
      view.scrollTop=atEnd?view.scrollHeight:priorTop;
      shownLog=next;lastLogId=forId;
      node('task-log-count').textContent=(next?next.trimEnd().split(/\r?\n/).length:0)+
        ' '+(getLang()==='en'?'lines':'行');
    }
  } catch (_) {
    if(req===activeLogRequest && forId===selectedId){
      node('task-log-content').textContent=tx('日志不存在、尚未生成，或已按三次保留规则清理。');
      node('task-log-count').textContent='0 '+(getLang()==='en'?'lines':'行');
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
      checkedOwnership=true;
    }
    const [task,history]=await Promise.all([get(apiTask),get(apiTask+'/history')]);
    renderTask(task);
    const runs=Array.isArray(history.runs)?history.runs:[];
    pickLatest(runs);
    renderHistory(runs);
    if(logOpted && node('task-log-auto').checked) await readLog();
  }catch(error){message(error.message);}
  finally{polling=false;}
}
async function act(action) {
  if(!checkedOwnership || actionBusy)return;
  const confirmed=window.confirm(tx(action==='run'?
    '点击确定会在 PerchFlow 所在电脑上执行真实 Python 脚本。继续？':'停止当前运行的 Python 脚本？'));
  if(!confirmed)return;
  actionBusy=true;
  try {
    const result=await get(apiTask+(action==='run'?'/run':'/stop'),{method:'POST'});
    if(result.already_finished) message('任务已经结束，状态已同步。');
    if(action==='run' && result.run_id) {
      followLatest=true;
      selectedId=result.run_id;
      logReset();
    }
    await poll();
  }catch(error){message(error.message);}
  finally{actionBusy=false;await poll();}
}
async function loadFiles() {
  if(fileLoading)return;
  fileLoading=true;
  const list=node('task-linked-files');
  list.replaceChildren(make('p','pf-taskdetail-notice',tx('正在加载…')));
  try {
    const result=await get(apiFiles+'/files');
    list.replaceChildren();
    const files=Array.isArray(result.files)?result.files:[];
    if(!files.length)list.append(make('p','pf-taskdetail-notice',
      tx('未发现可识别的文件，动态计算的路径需要人工检查。')));
    for(const file of files){
      const row=make('div','pf-taskdetail-file'),left=make('div','pf-taskdetail-file-info');
      left.append(make('div','pf-taskdetail-file-name',file.resolved_path));
      const origin={code_candidate:'代码候选',registered_input:'项目输入',
        registered_output:'项目输出'}[file.source]||file.source;
      left.append(make('div','pf-taskdetail-file-meta',tx(origin)+' · '+
        tx(file.exists?'存在':'未找到')+(file.size_bytes==null?'':' · '+file.size_bytes+' B')));
      if(file.inferred)left.append(make('small','pf-taskdetail-notice',
        tx('检测到的路径只是代码静态候选，并非确定的实际文件依赖。')));
      const open=make('button','pf-taskdetail-btn',tx('打开文件'));
      open.type='button';open.disabled=!file.exists;
      open.addEventListener('click',async()=>{
        if(!window.confirm(tx('将在运行 PerchFlow 的 Windows 电脑上用默认程序打开该文件。确认？')))return;
        try {
          const response=await get(apiFiles+'/open-file',{method:'POST',
            body:JSON.stringify({path:file.resolved_path,confirmed:true})});
          if(response.requested)message('仅支持本机 Windows 交互桌面；服务模式可能无法显示应用窗口。');
        }catch(err){message(err.message);}
      });
      row.append(left,open);list.append(row);
    }
  }catch(error){list.replaceChildren(make('p','pf-taskdetail-notice',error.message));}
  finally{fileLoading=false;}
}
document.addEventListener('DOMContentLoaded',()=>{
  localizeStatic();
  document.addEventListener('perchflow:language-changed',()=>{
    localizeStatic();
    if(latestTask)renderTask(latestTask);
    renderHistory(historyRuns);
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
  });  node('task-run').addEventListener('click',()=>{void act('run');});
  node('task-stop').addEventListener('click',()=>{void act('stop');});
  node('task-log-show').addEventListener('click',()=>{
    logOpted=true;
    void readLog();
  });
  node('task-files-refresh').addEventListener('click',()=>{void loadFiles();});
  if(!taskName || !projectName) {message('Missing task or project name');return;}
  void poll();
  void loadFiles();
  setInterval(()=>{void poll();},2000);
});
