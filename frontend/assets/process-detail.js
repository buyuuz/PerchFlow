const params = new URLSearchParams(window.location.search);
const procName = params.get('name') || '';
let liveOn = true;
let eventSource = null;

const STATUS_LABEL = { running: '运行中', stopped: '已停止', crashed: '已崩溃', restarting: '重启中' };

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

function fmtUptime(secs) {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  return `${h}h ${m}m ${s}s`;
}

async function loadProcess() {
  const resp = await api(`/api/processes/${encodeURIComponent(procName)}`);
  if (!resp.ok) return;
  const p = await resp.json();
  document.getElementById('proc-name').textContent = p.name;
  document.getElementById('proc-status').textContent = STATUS_LABEL[p.status] || p.status;
  document.getElementById('proc-pid').textContent = p.pid != null ? p.pid : '---';
  document.getElementById('proc-cpu').textContent = p.cpu_percent.toFixed(1) + '%';
  document.getElementById('proc-mem').textContent = Math.round(p.mem_bytes / 1024 / 1024) + ' MB';
  document.getElementById('proc-uptime').textContent = fmtUptime(p.uptime_secs);
  document.getElementById('proc-cmd').textContent = p.cmd;
}

function renderLine(line) {
  return `<div class="flex gap-4 hover:bg-white/[0.02] px-1 -mx-1 rounded-sm transition-colors"><span class="text-on-surface-variant break-all">${escapeHtml(line)}</span></div>`;
}

async function loadRecentLogs() {
  const lines = document.getElementById('lines-select').value;
  const data = await apiJson(`/api/processes/${encodeURIComponent(procName)}/logs?lines=${lines}`);
  const el = document.getElementById('log-content');
  el.innerHTML = data.lines.map(renderLine).join('');
  el.scrollTop = el.scrollHeight;
}

function startStream() {
  if (eventSource) return;
  const token = getToken();
  eventSource = new EventSource(`/api/logs/stream/${encodeURIComponent(procName)}?token=${encodeURIComponent(token)}`);
  eventSource.onmessage = (ev) => {
    const data = JSON.parse(ev.data);
    if (data.ended) {
      // 进程进入终态,SSE 流已由服务端结束。主动关闭 EventSource 避免重连,
      // 并在日志区底部追加一行说明。
      stopStream();
      const el = document.getElementById('log-content');
      const notice = document.createElement('div');
      notice.className = 'text-on-surface-variant italic px-1 py-1';
      notice.textContent = `— 日志流已结束 (${data.status}) —`;
      el.appendChild(notice);
      el.scrollTop = el.scrollHeight;
      return;
    }
    const el = document.getElementById('log-content');
    el.innerHTML = data.lines.map(renderLine).join('');
    el.scrollTop = el.scrollHeight;
  };
  eventSource.onerror = () => {
    // 服务端关闭流后 EventSource 会触发 onerror 并自动尝试重连。
    // 这里主动 close 阻止重连(已通过 ended 事件得知流应结束)。
    stopStream();
  };
}

function stopStream() {
  if (eventSource) { eventSource.close(); eventSource = null; }
}

function toggleLive() {
  liveOn = !liveOn;
  const toggle = document.getElementById('live-toggle');
  const track = toggle.querySelector('.toggle-track');
  const thumb = toggle.querySelector('.toggle-thumb');
  track.classList.toggle('bg-primary/20', liveOn);
  thumb.classList.toggle('translate-x-4', liveOn);
  if (liveOn) startStream(); else stopStream();
}

async function act(action) {
  await api(`/api/processes/${encodeURIComponent(procName)}/${action}`, { method: 'POST' });
  await loadProcess();
  // start/restart 会从 Stopped 终态切回 Running,之前 SSE 已被 ended 事件关闭,
  // 这里必须重建连接,否则日志区不刷新(需 F5 才能看到新日志)。
  // stop 会让服务端推 ended 后关流,前端 onmessage 会自动 stopStream,无需这里处理。
  if ((action === 'start' || action === 'restart') && liveOn) {
    stopStream();
    startStream();
  }
}

document.addEventListener('DOMContentLoaded', () => {
  document.querySelector('#live-toggle .toggle-track').classList.add('bg-primary/20');
  document.querySelector('#live-toggle .toggle-thumb').classList.add('translate-x-4');
  loadProcess();
  loadRecentLogs();
  startStream();
  setInterval(loadProcess, 3000);
});
