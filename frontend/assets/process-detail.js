const params = new URLSearchParams(window.location.search);
const procName = params.get('name') || '';
let liveOn = true;
let eventSource = null;
// 已渲染日志的最大全局序号(ring buffer 中每条日志带单调递增 seq)。
// 清屏时把它设为当前最新 seq,之后 live 推流只追加"清屏之后"的新日志。
let renderedSeq = -1;

function statusLabel(status) {
  const label = t('status.' + status);
  return label !== 'status.' + status ? label : status;
}

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
  document.getElementById('proc-status').textContent = statusLabel(p.status);
  document.getElementById('proc-pid').textContent = p.pid != null ? p.pid : '---';
  document.getElementById('proc-cpu').textContent = p.cpu_percent.toFixed(1) + '%';
  document.getElementById('proc-mem').textContent = Math.round(p.mem_bytes / 1024 / 1024) + ' MB';
  document.getElementById('proc-uptime').textContent = fmtUptime(p.uptime_secs);
  document.getElementById('proc-cmd').textContent = p.cmd;
}

function renderLine(line) {
  return `<div class="flex gap-4 hover:bg-white/[0.02] px-1 -mx-1 rounded-sm transition-colors"><span class="text-on-surface-variant break-all">${escapeHtml(line)}</span></div>`;
}

function renderLineSeq(item) {
  return `<div class="flex gap-4 hover:bg-white/[0.02] px-1 -mx-1 rounded-sm transition-colors"><span class="text-on-surface-variant break-all">${escapeHtml(item.text)}</span></div>`;
}

async function loadRecentLogs() {
  const lines = document.getElementById('lines-select').value;
  const data = await apiJson(`/api/processes/${encodeURIComponent(procName)}/logs?lines=${lines}`);
  const el = document.getElementById('log-content');
  el.innerHTML = data.lines.map(renderLineSeq).join('');
  renderedSeq = data.seq - 1;
  el.scrollTop = el.scrollHeight;
}

// 清屏:清空日志区,并把增量基线推进到当前最新 seq——之后 live 推流只显示新日志。
function clearLogs() {
  const el = document.getElementById('log-content');
  el.innerHTML = '';
  el.scrollTop = 0;
}

// 距底部多少像素内视为"正在看最新的",此时新日志到达自动跟随滚动到最底;
// 用户上翻查看历史时保持不动,不被每秒推流打扰。
const AUTO_SCROLL_THRESHOLD = 40;

function shouldAutoScroll(el) {
  return el.scrollHeight - el.scrollTop - el.clientHeight < AUTO_SCROLL_THRESHOLD;
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
      notice.textContent = t('log.stream_ended', {status: data.status});
      el.appendChild(notice);
      if (shouldAutoScroll(el)) el.scrollTop = el.scrollHeight;
      return;
    }
    const el = document.getElementById('log-content');
    if (data.seq <= renderedSeq) {
      // buffer 被重置(进程重启,seq 从 0 重新计数):全量重渲染,避免停留空白。
      el.innerHTML = data.lines.map(renderLineSeq).join('');
    } else {
      // 常规增量:只追加 seq 大于已渲染最大序号的行(清屏后据此只显示新日志)。
      const fresh = data.lines.filter(l => l.seq > renderedSeq);
      el.innerHTML += fresh.map(renderLineSeq).join('');
    }
    renderedSeq = data.seq - 1;
    // 仅在用户位于底部附近时自动跟随,上翻查看历史时保持当前位置。
    if (shouldAutoScroll(el)) el.scrollTop = el.scrollHeight;
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
