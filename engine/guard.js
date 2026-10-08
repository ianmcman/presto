// Origin allowlist for navigation, IPC senders and permissions.
const HOSTS = ['apple.com', 'icloud.com', 'mzstatic.com'];
const PERMISSIONS = new Set(['media', 'mediaKeySystem']);

function allowed(url) {
  if (url === 'about:blank') return true;
  let u;
  try { u = new URL(url); } catch { return false; }
  return u.protocol === 'https:' && HOSTS.some((h) => u.hostname === h || u.hostname.endsWith('.' + h));
}
// origin + path only, never query or fragment
const safe = (url) => { try { const u = new URL(url); return u.origin + u.pathname; } catch { return '<bad-url>'; } };

module.exports = { allowed, safe, PERMISSIONS };
