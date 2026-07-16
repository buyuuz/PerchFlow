const STATUS_STYLE = {
  running: { label: 'RUNNING', badge: 'bg-green-500/10 text-green-400 border border-green-500/20', strip: 'bg-green-500/50' },
  stopped: { label: 'STOPPED', badge: 'bg-surface-container-highest text-on-surface-variant border border-white/10', strip: 'bg-surface-container-highest' },
  crashed: { label: 'CRASHED', badge: 'bg-error/10 text-error border border-error/20', strip: 'bg-error/80' },
  restarting: { label: 'RESTARTING', badge: 'bg-tertiary/10 text-tertiary border border-tertiary/20', strip: 'bg-tertiary/70' },
};

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

function fmtUptime(secs) {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m ${secs % 60}s`;
}

function renderCard(p) {
  const style = STATUS_STYLE[p.status] || STATUS_STYLE.stopped;
  const pidText = p.pid != null ? `PID: ${p.pid}` : 'PID: ---';
  const memMb = Math.round(p.mem_bytes / 1024 / 1024);
  return `
<div class="glass-card rounded-lg p-5 flex flex-col relative overflow-hidden group">
<div class="flex justify-between items-start mb-4">
<div>
<h2 class="font-headline-md text-headline-md text-on-surface">${escapeHtml(p.name)}</h2>
<div class="flex items-center gap-2 mt-1">
<span class="inline-flex items-center px-2 py-0.5 rounded text-[11px] font-mono-label font-medium ${style.badge}">${style.label}</span>
<span class="font-mono-data text-mono-data text-on-surface-variant">${pidText}</span>
</div>
</div>
</div>
<div class="grid grid-cols-4 gap-2 mb-4 bg-surface-container-lowest/50 rounded p-3 border border-white/5">
<div class="flex flex-col">
<span class="text-[10px] text-on-surface-variant uppercase tracking-wider mb-1">CPU</span>
<span class="font-mono-data text-mono-data text-on-surface">${p.cpu_percent.toFixed(1)}%</span>
</div>
<div class="flex flex-col">
<span class="text-[10px] text-on-surface-variant uppercase tracking-wider mb-1">MEM</span>
<span class="font-mono-data text-mono-data text-on-surface">${memMb} MB</span>
</div>
<div class="flex flex-col">
<span class="text-[10px] text-on-surface-variant uppercase tracking-wider mb-1">UPTIME</span>
<span class="font-mono-data text-mono-data text-on-surface">${fmtUptime(p.uptime_secs)}</span>
</div>
<div class="flex flex-col">
<span class="text-[10px] text-on-surface-variant uppercase tracking-wider mb-1">RESTARTS</span>
<span class="font-mono-data text-mono-data text-on-surface">${p.restart_count}</span>
</div>
</div>
<div class="mb-4">
<div class="font-mono-data text-mono-data text-on-surface-variant bg-[#000000] bg-opacity-40 p-2 rounded border border-white/5 truncate" title="${escapeHtml(p.cmd)}">
<span class="text-primary/70 mr-2">$</span>${escapeHtml(p.cmd)}
</div>
</div>
<div class="mt-auto pt-2 flex justify-end gap-2 border-t border-white/5">
<button class="w-8 h-8 rounded flex items-center justify-center text-on-surface-variant hover:text-green-400 hover:bg-white/5 transition-colors" onclick="act('${encodeURIComponent(p.name)}','start')" title="Start">
<span class="material-symbols-outlined text-[18px]">play_arrow</span>
</button>
<button class="w-8 h-8 rounded flex items-center justify-center text-on-surface-variant hover:text-amber-400 hover:bg-white/5 transition-colors" onclick="act('${encodeURIComponent(p.name)}','restart')" title="Restart">
<span class="material-symbols-outlined text-[18px]">refresh</span>
</button>
<button class="w-8 h-8 rounded flex items-center justify-center text-on-surface-variant hover:text-red-400 hover:bg-white/5 transition-colors" onclick="act('${encodeURIComponent(p.name)}','stop')" title="Stop">
<span class="material-symbols-outlined text-[18px]">stop</span>
</button>
<div class="w-[1px] h-4 bg-white/10 my-auto mx-1"></div>
<button class="w-8 h-8 rounded flex items-center justify-center text-primary bg-primary/10 hover:bg-primary/20 border border-primary/20 transition-colors" onclick="window.location.href='/process-detail?name=${encodeURIComponent(p.name)}'" title="View Logs">
<span class="material-symbols-outlined text-[18px]">terminal</span>
</button>
</div>
<div class="absolute left-0 top-0 bottom-0 w-1 ${style.strip}"></div>
</div>`;
}

async function loadProcesses() {
  const { processes } = await apiJson('/api/processes');
  const grid = document.getElementById('proc-grid');
  grid.innerHTML = processes.map(renderCard).join('');
}

async function act(name, action) {
  await api(`/api/processes/${name}/${action}`, { method: 'POST' });
  loadProcesses();
}

function openAddModal() {
  document.getElementById('add-name').value = '';
  document.getElementById('add-cmd').value = '';
  document.getElementById('add-args').value = '';
  document.getElementById('add-autostart').checked = false;
  document.getElementById('add-err').classList.add('hidden');
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
  const body = {
    name, cmd, args, autostart,
    restart: { policy: 'on-failure', backoff_secs: 2, backoff_max_secs: 60, max_retries: 0 },
  };
  const resp = await api('/api/processes', { method: 'POST', body: JSON.stringify(body) });
  const result = await resp.json();
  if (!result.ok) {
    errEl.textContent = result.error || t('error.save_failed');
    errEl.classList.remove('hidden');
    return;
  }
  closeAddModal();
  loadProcesses();
}

document.addEventListener('DOMContentLoaded', () => { loadProcesses(); setInterval(loadProcesses, 3000); });
