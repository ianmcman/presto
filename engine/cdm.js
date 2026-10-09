// CDM status helpers: pure so they can be tested without Electron.
const CDM_TIMEOUT_MS = 120000;

// ComponentsError carries errors[].detail {id, status}; anything else falls back to .message.
function cdmMessage(e) {
  const parts = Array.isArray(e?.errors) ? e.errors.map((x) => `${x?.detail?.id ?? '?'}: ${x?.detail?.status ?? '?'}`) : [];
  return (parts.join('; ') || e?.message || 'unknown error').slice(0, 300);
}

function withTimeout(p, ms) {
  let t;
  const timer = new Promise((_, rej) => { t = setTimeout(() => rej(new Error(`timed out after ${ms / 1000} s waiting for the Widevine CDM`)), ms); });
  return Promise.race([p, timer]).finally(() => clearTimeout(t));
}

module.exports = { CDM_TIMEOUT_MS, cdmMessage, withTimeout };
