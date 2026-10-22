/* PerchFlow Docker V1 — local Docker CLI only, all data from authenticated API. */
'use strict';
(() => {
  const $ = id => document.getElementById(id);
  const state = { containers: [], selected: null, loading: false, detailLoading: null, logsLoading: null, busy: false, lastOk: false, refreshQueued: false, operationError: null };
  const t = (zh,en) => typeof getLang === 'function' && getLang() === 'en' ? en : zh;
  function formatUptime(s) {
    if (s === null || s === undefined) return '—';
    const n = Math.max(0, Number(s) || 0);
    if (n < 60) return Math.floor(n) + 's';
    if (n < 3600) return Math.floor(n / 60) + 'm';
    if (n < 86400) return Math.floor(n / 3600) + 'h ' + Math.floor(n % 3600 / 60) + 'm';
    return Math.floor(n / 86400) + 'd ' + Math.floor(n % 86400 / 3600) + 'h';
  }
  function el(tag, cls, value) {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (value !== undefined) node.textContent = String(value);
    return node;
  }
  function showError(msg) {
    const e = $('docker-error');
    e.hidden = !msg;
    e.textContent = msg || '';
  }
  function badge(status, exitCode) {
    const values = {
      running:[t('运行中','Running'),'is-running'],
      exited:[t('已停止','Stopped'),''],
      created:[t('已创建','Created'),''],
      restarting:[t('重启中','Restarting'),'is-running'],
      paused:[t('已暂停','Paused'),''],
      dead:[t('异常退出','Dead'),'is-bad'],
      removing:[t('移除中','Removing'),''],
    };
    if (status==='exited' && Number(exitCode||0)!==0) return el('span','pf-docker-status is-bad',t('异常退出','Exited with error'));
    const [text,cls] = values[status] || [status||'—','is-bad'];
    return el('span', 'pf-docker-status '+cls, text);
  }
  function unavailable(message) {
    state.lastOk=false;
    state.containers=[];
    $('docker-total').textContent='—';$('docker-running').textContent='—';$('docker-stopped').textContent='—';
    $('docker-updated').textContent=t('未连接 Docker Engine','Docker Engine unavailable');
    $('docker-top-label').textContent=t('未连接','Disconnected');
    showError(message || t('Docker CLI 或 Docker Engine 当前无法访问','Cannot access Docker CLI or Docker Engine'));
    $('docker-tbody').replaceChildren(el('tr','',undefined));
    const td=el('td','',t('无法获取真实容器状态','Unable to fetch live container state'));
    td.colSpan=6;$('docker-tbody').firstChild.append(td);
  }
  async function refresh() {
    if (state.loading) { state.refreshQueued=true; return; }
    if (state.busy || document.hidden) return;
    state.loading=true;
    $('docker-refresh').disabled=true;
    try {
      const data = await apiJson('/api/docker/containers');
      if (!data.ok) {
        unavailable(data.message);
        if (state.selected) {
          $('docker-start').disabled=true;$('docker-stop').disabled=true;$('docker-restart').disabled=true;
        }
        return;
      }
      showError(state.operationError || '');
      state.lastOk=true;
      state.containers=Array.isArray(data.containers)?data.containers:[];
      const active=state.containers.filter(x=>x.status==='running'||x.status==='restarting').length;
      $('docker-total').textContent=state.containers.length;
      $('docker-running').textContent=active;
      $('docker-stopped').textContent=state.containers.length-active;
      $('docker-top-label').textContent=t('已连接','Connected');
      $('docker-updated').textContent=t('最近更新：','Updated: ')+new Date(data.updated_at||Date.now()).toLocaleTimeString();
      const tbody=$('docker-tbody');tbody.replaceChildren();
      if (!state.containers.length) {
        const td=el('td','',t('Docker Engine 已连接，但没有容器','Docker is connected, but no containers exist'));
        td.colSpan=6;const row=el('tr');row.append(td);tbody.append(row);
      }
      for (const c of state.containers) {
        const row=el('tr');row.dataset.id=c.id;row.tabIndex=0;
        const first=el('td');
        first.append(el('div','pf-docker-name',c.name||c.short_id||c.id));
        first.append(el('small','',String(c.image||'')+' · '+String(c.short_id||'')));
        row.append(first);
        const second=el('td');second.append(badge(c.status,c.exit_code));row.append(second);
        row.append(el('td','',c.cpu_percent===null?t('—','—'):Number(c.cpu_percent).toFixed(2)+'%'));
        row.append(el('td','',c.memory_mib===null?'—':Number(c.memory_mib).toFixed(1)+' MiB'));
        row.append(el('td','',formatUptime(c.uptime_seconds)));
        const last=el('td');last.append(el('span','material-symbols-outlined','chevron_right'));row.append(last);
        row.addEventListener('click',()=>void openDetail(c.id));
        row.addEventListener('keydown',ev=>{if(ev.key==='Enter'){ev.preventDefault();void openDetail(c.id);}});
        tbody.append(row);
      }
      if (state.selected) {
        const selected=state.containers.find(c=>c.id===state.selected);
        if (!selected) {
          showError(t('该容器已经不存在，请返回列表','Container no longer exists; return to list'));
          $('docker-start').disabled=true;$('docker-stop').disabled=true;$('docker-restart').disabled=true;
        } else {
          updateButtons(selected);
          updateOverview(selected);
        }
      }
    } catch (e) {
      if (e.message !== 'unauthorized') unavailable(e.message);
    } finally {
      state.loading=false;$('docker-refresh').disabled=false;
      if (state.refreshQueued && !state.busy) {state.refreshQueued=false;void refresh();}
    }
  }
  function updateButtons(c) {
    const running=c.status==='running'||c.status==='restarting';
    $('docker-start').disabled=state.busy||running||!state.lastOk;
    $('docker-stop').disabled=state.busy||!running||!state.lastOk;
    $('docker-restart').disabled=state.busy||!state.lastOk;
  }
  function item(label,val) {
    const e=el('div','pf-docker-info');
    e.append(el('small','',label),el('strong','',val===null||val===undefined||val===''?'—':val));
    return e;
  }
  function updateOverview(c,info) {
    $('docker-name').textContent=c.name||'—';$('docker-image').textContent=c.image||'—';
    const root=$('docker-info');root.replaceChildren();
    root.append(item(t('运行状态','Status'), badge(c.status,c.exit_code).textContent));
    root.append(item(t('CPU 使用率','CPU'),c.cpu_percent===null?'—':Number(c.cpu_percent).toFixed(2)+'%'));
    root.append(item(t('内存使用量','Memory'),c.memory_mib===null?'—':Number(c.memory_mib).toFixed(1)+' MiB'));
    root.append(item(t('运行时长','Uptime'),formatUptime(c.uptime_seconds)));
    root.append(item(t('容器 ID','Container ID'),c.id));
    root.append(item(t('镜像','Image'),c.image));
    root.append(item(t('启动时间','Started at'),formatTime(info?.started_at||c.started_at)));
    root.append(item(t('重启次数','Restart count'),info?.restart_count??c.restart_count??0));
    root.append(item(t('退出码','Exit code'),info?.exit_code??c.exit_code??'—'));
  }
  function formatTime(v) {
    if (!v||String(v).startsWith('0001-')) return '—';
    const d=new Date(v);return Number.isNaN(d.getTime())?String(v):d.toLocaleString();
  }
  async function openDetail(id) {
    if (!/^[a-fA-F0-9]{64}$/.test(id)) return;
    state.selected=id;
    $('docker-list-screen').hidden=true;$('docker-detail-screen').hidden=false;
    const c=state.containers.find(x=>x.id===id);
    if (c) {updateButtons(c);updateOverview(c);}
    else {updateButtons({status:'unknown'});$('docker-info').replaceChildren();}
    await Promise.all([refreshDetail(),refreshLogs()]);
    window.scrollTo({top:0,behavior:'auto'});
  }
  function back() {
    state.selected=null;$('docker-detail-screen').hidden=true;$('docker-list-screen').hidden=false;
    showError('');void refresh();
  }
  async function refreshDetail() {
    if (!state.selected||state.detailLoading===state.selected) return;
    const id=state.selected;state.detailLoading=id;
    try {
      const result=await apiJson('/api/docker/containers/'+encodeURIComponent(id));
      if (id!==state.selected) return;
      if (!result.ok) {showError(result.message);return;}
      const c=state.containers.find(x=>x.id===id);
      if (c) updateOverview(c,result.container);
    } catch(e){if(id===state.selected)showError(e.message);}
    finally {if (state.detailLoading===id)state.detailLoading=null;}
  }
  async function refreshLogs() {
    if (!state.selected||state.logsLoading===state.selected||!state.lastOk) return;
    const id=state.selected;
    state.logsLoading=id;$('docker-log-refresh').disabled=true;
    const pre=$('docker-log-text');
    const bottom=pre.scrollHeight-pre.clientHeight-pre.scrollTop<48;
    try {
      const n=Number($('docker-log-tail').value)||200;
      const result=await apiJson('/api/docker/containers/'+encodeURIComponent(id)+'/logs?tail='+n);
      if (id!==state.selected) return;
      if (!result.ok) {showError(result.message);return;}
      pre.textContent=result.logs||t('暂无日志输出','No log output');
      if (result.truncated) pre.textContent+='\n'+t('[显示内容超过限制，已截断]','[Output truncated]');
      if (bottom)pre.scrollTop=pre.scrollHeight;
    } catch(e){if(id===state.selected)showError(e.message);}
    finally{if(state.logsLoading===id){state.logsLoading=null;$('docker-log-refresh').disabled=false;}}
  }
  async function doOperation(kind) {
    const id=state.selected;
    if (!id||state.busy||!state.lastOk) return;
    const current=state.containers.find(x=>x.id===id);
    if (!current) return;
    if (kind==='stop'||kind==='restart') {
      const verb=kind==='stop'?t('停止','stop'):t('重启','restart');
      const ok=await PerchDialog.confirm(t('确定要','Are you sure you want to ')+verb+' '+current.name+'？',{
        title:kind==='stop'?t('停止 Docker 容器','Stop Docker container'):t('重启 Docker 容器','Restart Docker container'),
        ok:verb,danger:kind==='stop'
      });
      if (!ok||state.selected!==id) return;
    }
    state.busy=true;state.operationError=null;updateButtons(current);showError('');
    try {
      const result=await apiJson('/api/docker/containers/'+encodeURIComponent(id)+'/'+kind,{method:'POST'});
      if (!result.ok) state.operationError=result.message||result.error||'Docker operation failed';
    } catch(e){state.operationError=e.message;}
    finally{
      state.busy=false;
      await refresh();
      if (state.selected===id) {await refreshDetail();await refreshLogs();}
      if (state.operationError) showError(state.operationError);
    }
  }
  $('docker-refresh').addEventListener('click',()=>void refresh());
  $('docker-back').addEventListener('click',back);
  $('docker-start').addEventListener('click',()=>void doOperation('start'));
  $('docker-stop').addEventListener('click',()=>void doOperation('stop'));
  $('docker-restart').addEventListener('click',()=>void doOperation('restart'));
  $('docker-log-refresh').addEventListener('click',()=>void refreshLogs());
  $('docker-log-tail').addEventListener('change',()=>void refreshLogs());
  setInterval(()=>{
    if (document.hidden) return;
    void refresh();
    if (state.selected&&$('docker-follow').checked) void refreshLogs();
  },5000);
  document.addEventListener('visibilitychange',()=>{
    if(!document.hidden){void refresh();if(state.selected&&$('docker-follow').checked)void refreshLogs();}
  });
  void refresh();
})();
