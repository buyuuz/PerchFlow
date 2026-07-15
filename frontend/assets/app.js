const TOKEN_KEY = 'perch_token';

function getToken() { return localStorage.getItem(TOKEN_KEY) || ''; }
function setToken(t) { localStorage.setItem(TOKEN_KEY, t); }

async function api(path, opts = {}) {
  const headers = Object.assign({ 'Authorization': 'Bearer ' + getToken() }, opts.headers || {});
  if (opts.body && !headers['Content-Type']) headers['Content-Type'] = 'application/json';
  const resp = await fetch(path, Object.assign({}, opts, { headers }));
  if (resp.status === 401) { window.location.href = '/'; throw new Error('unauthorized'); }
  return resp;
}

async function apiJson(path, opts) { return (await api(path, opts)).json(); }
