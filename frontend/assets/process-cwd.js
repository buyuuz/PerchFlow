/* PerchFlow working-directory input: optional, host-validated and never auto-overwritten.
   The browser cannot inspect host paths; the authenticated Rust API validates them. */
let cwdSuggestedDirectory = '';
let cwdValidationRevision = 0;

function normalizedCwdText(raw) {
  const value = String(raw || '').trim();
  if (value.length >= 2 && value.startsWith('"') && value.endsWith('"')) {
    return value.slice(1, -1).trim();
  }
  return value;
}

function isAbsoluteWindowsDirectory(value) {
  const path = normalizedCwdText(value).replace(/\//g, '\\');
  return /^[A-Za-z]:\\/.test(path) || /^\\\\[^\\]+\\[^\\]+(?:\\|$)/.test(path);
}

function directorySuggestedByExe(command) {
  let executable = normalizedCwdText(command);
  // A bare command on PATH or an interpreter does NOT identify the project directory.
  if (!/\.exe$/i.test(executable)) return '';
  executable = executable.replace(/\//g, '\\');
  if (!isAbsoluteWindowsDirectory(executable)) return '';
  const end = executable.lastIndexOf('\\');
  if (end < 0) return '';
  if (/^[A-Za-z]:\\/.test(executable) && end === 2) {
    return executable.slice(0, end + 1); // e.g. C:\app.exe -> C:\
  }
  return executable.slice(0, end);
}

function cwdText(key) {
  return typeof t === 'function' ? t(key) : key;
}

function displayCwdStatus(key, tone = '') {
  const el = document.getElementById('cwd-validation');
  if (!el) return;
  el.dataset.statusKey = key || '';
  el.dataset.tone = tone;
  el.hidden = !key;
  el.textContent = key ? cwdText(key) : '';
}

function updateCwdSuggestion() {
  const cmdInput = document.getElementById('add-cmd');
  const cwdInput = document.getElementById('add-cwd');
  const area = document.getElementById('cwd-suggestion');
  const value = document.getElementById('cwd-suggest-value');
  if (!cmdInput || !cwdInput || !area || !value) return;
  cwdSuggestedDirectory = directorySuggestedByExe(cmdInput.value);
  // Offer the directory as an explicit choice: e.g. python.exe lives in a
  // different directory from the project. Never fill/overwrite cwd silently.
  area.hidden = !cwdSuggestedDirectory || Boolean(normalizedCwdText(cwdInput.value));
  value.textContent = cwdSuggestedDirectory;
  value.title = cwdSuggestedDirectory;
}

function applySuggestedCwd() {
  const cwdInput = document.getElementById('add-cwd');
  if (!cwdInput || !cwdSuggestedDirectory) return;
  cwdInput.value = cwdSuggestedDirectory;
  ++cwdValidationRevision;
  displayCwdStatus('');
  updateCwdSuggestion();
  cwdInput.focus();
}

function resetWorkingDirectoryField(value = '') {
  const input = document.getElementById('add-cwd');
  if (!input) return;
  input.value = String(value || '');
  ++cwdValidationRevision;
  displayCwdStatus('');
  updateCwdSuggestion();
}

function getWorkingDirectoryInput() {
  return normalizedCwdText(document.getElementById('add-cwd')?.value);
}

async function validateWorkingDirectory() {
  const cwd = getWorkingDirectoryInput();
  const seq = ++cwdValidationRevision;
  if (!cwd) {
    displayCwdStatus('');
    return true;
  }
  if (cwd.length > 1024) {
    displayCwdStatus('error.cwd_too_long', 'error');
    return false;
  }
  if (!isAbsoluteWindowsDirectory(cwd)) {
    displayCwdStatus('error.cwd_not_absolute', 'error');
    return false;
  }

  displayCwdStatus('cwd.checking', 'checking');
  try {
    const res = await api('/api/processes/validate-cwd', {
      method: 'POST',
      body: JSON.stringify({ cwd }),
    });
    const result = await res.json();
    if (seq !== cwdValidationRevision) return false;
    if (!res.ok || !result.ok) {
      const known = {
        cwd_too_long: 'error.cwd_too_long',
        cwd_not_absolute: 'error.cwd_not_absolute',
        cwd_not_found: 'error.cwd_not_found',
        cwd_not_directory: 'error.cwd_not_directory',
        cwd_inaccessible: 'error.cwd_inaccessible',
      };
      displayCwdStatus(known[result.error] || 'error.cwd_check_failed', 'error');
      return false;
    }
    displayCwdStatus('cwd.valid', 'success');
    return true;
  } catch (error) {
    if (seq !== cwdValidationRevision) return false;
    console.warn('Working directory validation unavailable:', error);
    displayCwdStatus('error.cwd_check_failed', 'error');
    return false;
  }
}

document.addEventListener('DOMContentLoaded', () => {
  const cmd = document.getElementById('add-cmd');
  const cwd = document.getElementById('add-cwd');
  cmd?.addEventListener('input', updateCwdSuggestion);
  cwd?.addEventListener('input', () => {
    ++cwdValidationRevision;
    displayCwdStatus('');
    updateCwdSuggestion();
  });
  cwd?.addEventListener('blur', () => {
    if (!document.getElementById('add-modal')?.classList.contains('hidden')) {
      void validateWorkingDirectory();
    }
  });
  document.addEventListener('perchflow:language-changed', () => {
    const key = document.getElementById('cwd-validation')?.dataset.statusKey;
    if (key) displayCwdStatus(key, document.getElementById('cwd-validation').dataset.tone);
  });
});
