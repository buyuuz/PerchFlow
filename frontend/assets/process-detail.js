/* PerchFlow process detail — preview-friendly presentation and existing API flow.
   The same endpoints power the production UI; visual edits do not change API semantics. */
const params = new URLSearchParams(window.location.search);
const procName = params.get('name') || '';
let liveOn = true;
let eventSource = null;
let lastProcess = null;
let renderedSeq = -1;

// If the log stream's global sequence resets on restart, the viewer refreshes
// the display. Clearing the viewport intentionally does not change renderedSeq.
const AUTO_SCROLL_THRESHOLD = 40;

function statusLabel(status) {
  const label = t('status.' + status);
  return label !== 'status.' + status ? label : status;
}

function escapeHtml(value) {
  return String(value ?? '').replace(/[&<>"']/g, c => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  }[c]));
}

function fmtUptime(value) {
  const secs = Math.max(0, Math.floor(Number(value) || 0));
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  return d ? `${d}d ${h}h ${m}m` : `${h}h ${m}m ${s}s`;
}

function renderLineSeq(item) {
  const line = String(item?.text ?? '');
  let tone = '';
  if (/\b(?:ERROR|FATAL|CRITICAL)\b|\[ERR\]/i.test(line)) tone = ' pf-log-line--error';
  else if (/\b(?:WARN|WARNING)\b/i.test(line)) tone = ' pf-log-line--warn';
  else if (/\bINFO\b/i.test(line)) tone = ' pf-log-line--info';
  return `<div class="pf-log-line${tone}">${escapeHtml(line)}</div>`;
}

function updateLogCount() {
  const total = document.querySelectorAll('#log-content .pf-log-line').length;
  const count = document.getElementById('log-line-count');
  if (count) count.textContent = t('detail.log_count', {count: total});
}

function syncLiveIndicator() {
  const toggle = document.getElementById('live-toggle');
  toggle.classList.toggle('is-on', liveOn);
  toggle.setAttribute('aria-checked', String(liveOn));
  const mode = document.getElementById('log-view-mode');
  if (mode) {
    mode.textContent = t(liveOn ? 'detail.live_status' : 'detail.paused_status');
    mode.dataset.live = String(liveOn);
  }
}

function syncDetailLanguage() {
  document.querySelector('.pf-detail-back').setAttribute('aria-label', t('detail.back'));
  document.getElementById('detail-copy').setAttribute('aria-label', t('detail.copy_command'));
  if (lastProcess) renderProcess(lastProcess);
  const empty = document.getElementById('log-content');
  if (empty) empty.dataset.empty = t('detail.no_logs');
  updateLogCount();
  syncLiveIndicator();
}

function renderProcess(p) {
  const name = String(p.name || procName);
  document.getElementById('proc-name').textContent = name;
  document.getElementById('proc-name').title = name;
  const alias = document.getElementById('proc-alias-zh');
  alias.textContent = String(p.alias_zh || '').trim();
  alias.title = alias.textContent;
  alias.hidden = !alias.textContent;
  document.getElementById('log-process-name').textContent = name;
  const status = ['running','stopped','crashed','restarting'].includes(p.status) ? p.status : 'unknown';
  document.getElementById('proc-status-badge').dataset.status = status;
  document.getElementById('proc-status').textContent = statusLabel(status);
  document.getElementById('proc-pid').textContent = p.pid != null ? p.pid : '—';
  document.getElementById('proc-cpu').textContent = (Number(p.cpu_percent) || 0).toFixed(1) + '%';
  document.getElementById('proc-mem').textContent = Math.round(Math.max(0, Number(p.mem_bytes) || 0) / 1048576) + ' MB';
  document.getElementById('proc-uptime').textContent = fmtUptime(p.uptime_secs);
  document.getElementById('proc-cmd').textContent = String(p.cmd || '—');
  document.getElementById('proc-cmd').title = String(p.cmd || '');
  const running = status === 'running' || status === 'restarting';
  document.getElementById('detail-start').disabled = running;
  document.getElementById('detail-stop').disabled = !running;
}

async function loadProcess() {
  try {
    const resp = await api(`/api/processes/${encodeURIComponent(procName)}`);
    if (!resp.ok) throw new Error('Process request failed: HTTP ' + resp.status);
    lastProcess = await resp.json();
    renderProcess(lastProcess);
  } catch (error) {
    console.error('Unable to refresh process detail:', error);
  }
}

async function copyProcessCommand() {
  const text = String(lastProcess?.cmd || document.getElementById('proc-cmd').textContent || '');
  if (!text || text === '—') return;
  const button = document.getElementById('detail-copy');
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
      const result = document.execCommand('copy');
      field.remove();
      if (!result) throw new Error('Clipboard unavailable');
    }
    button.title = t('detail.copied');
    button.querySelector('.material-symbols-outlined').textContent = 'check';
    setTimeout(() => {
      if (!button.isConnected) return;
      button.title = t('detail.copy_command');
      button.querySelector('.material-symbols-outlined').textContent = 'content_copy';
    }, 1200);
  } catch (error) {
    console.error('Copy command failed:', error);
    button.title = t('detail.copy_failed');
  }
}

async function loadRecentLogs() {
  const lines = document.getElementById('lines-select').value;
  try {
    const data = await apiJson(`/api/processes/${encodeURIComponent(procName)}/logs?lines=${encodeURIComponent(lines)}`);
    const el = document.getElementById('log-content');
    const rows = Array.isArray(data.lines) ? data.lines : [];
    el.innerHTML = rows.map(renderLineSeq).join('');
    renderedSeq = Number(data.seq || 0) - 1;
    el.scrollTop = el.scrollHeight;
    updateLogCount();
  } catch (error) {
    console.error('Unable to load recent logs:', error);
  }
}

// Clears only the displayed history. Later live events append new lines.
function clearLogs() {
  const el = document.getElementById('log-content');
  el.innerHTML = '';
  el.scrollTop = 0;
  updateLogCount();
}

function shouldAutoScroll(el) {
  return el.scrollHeight - el.scrollTop - el.clientHeight < AUTO_SCROLL_THRESHOLD;
}

function startStream() {
  if (eventSource || !liveOn || !procName) return;
  const token = getToken();
  eventSource = new EventSource(`/api/logs/stream/${encodeURIComponent(procName)}?token=${encodeURIComponent(token)}`);
  eventSource.onmessage = ev => {
    let data;
    try { data = JSON.parse(ev.data); }
    catch (error) { console.warn('Invalid log stream event', error); return; }
    const el = document.getElementById('log-content');
    const autoscroll = shouldAutoScroll(el);

    if (data.ended) {
      stopStream();
      const notice = document.createElement('div');
      notice.className = 'pf-log-system';
      notice.textContent = t('log.stream_ended', {status: data.status});
      el.appendChild(notice);
      if (autoscroll) el.scrollTop = el.scrollHeight;
      return;
    }

    const rows = Array.isArray(data.lines) ? data.lines : [];
    const seq = Number(data.seq || 0);
    if (seq <= renderedSeq) {
      el.innerHTML = rows.map(renderLineSeq).join('');
    } else {
      const fresh = rows.filter(row => row.seq > renderedSeq);
      // Append as HTML only after encoding each line's text.
      el.insertAdjacentHTML('beforeend', fresh.map(renderLineSeq).join(''));
    }
    renderedSeq = seq - 1;
    updateLogCount();
    if (autoscroll) el.scrollTop = el.scrollHeight;
  };
  eventSource.onerror = () => {
    // Stop reconnect loops when the server terminates the stream.
    stopStream();
  };
}

function stopStream() {
  if (eventSource) { eventSource.close(); eventSource = null; }
}

function toggleLive() {
  liveOn = !liveOn;
  syncLiveIndicator();
  if (liveOn) startStream();
  else stopStream();
}

async function act(action) {
  const button = document.getElementById('detail-' + action);
  if (button) button.disabled = true;
  try {
    const resp = await api(`/api/processes/${encodeURIComponent(procName)}/${action}`, {method: 'POST'});
    if (!resp.ok) throw new Error('Process action failed: HTTP ' + resp.status);
    const result = await resp.json();
    if (result.ok === false) throw new Error(result.error || 'Action failed');
    await loadProcess();
    if ((action === 'start' || action === 'restart') && liveOn) {
      stopStream();
      startStream();
    }
  } catch (error) {
    console.error('Process action failed:', error);
    await loadProcess();
  } finally {
    if (button && action === 'restart') button.disabled = false;
  }
}

document.addEventListener('DOMContentLoaded', () => {
  document.getElementById('log-process-name').textContent = procName || 'process';
  document.getElementById('proc-name').textContent = procName || '—';
  document.addEventListener('perchflow:language-changed', syncDetailLanguage);
  syncDetailLanguage();
  void loadProcess();
  void loadRecentLogs();
  startStream();
  setInterval(loadProcess, 3000);
  window.addEventListener('pagehide', stopStream);
});
