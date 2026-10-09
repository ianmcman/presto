// presto-engine: castlabs ECS hosting music.apple.com, speaking presto-ipc over a Unix socket.
// All logging goes to stderr; stdout stays silent.
const fs = require('fs');
const net = require('net');
const os = require('os');
const path = require('path');
const readline = require('readline');
const { app, BrowserWindow, ipcMain, session, components } = require('electron');

const { allowed, safe, PERMISSIONS } = require('./guard');
const { resolveBridge } = require('./bridge-path');
const { windowAction } = require('./window-policy');
const { CDM_TIMEOUT_MS, cdmMessage, withTimeout } = require('./cdm');

const log = (...a) => console.error('presto-engine', ...a);

// --- args ---
const argv = process.argv.slice(2);
const val = (name) => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : undefined; };
const has = (name) => argv.includes(name);
const args = {
  socket: val('--socket') ?? process.env.PRESTO_SOCKET,
  profile: val('--profile') ?? process.env.PRESTO_PROFILE,
  url: val('--url') ?? 'https://music.apple.com',
  show: has('--show'),
  noAutoplaySwitch: has('--no-autoplay-switch'),
  allowThrottling: has('--allow-throttling'),
  keepMediaSession: has('--keep-media-session'),
  diag: has('--diag'),
};
if (!args.socket || !args.profile) {
  console.error('presto-engine: --socket and --profile (or PRESTO_SOCKET / PRESTO_PROFILE) are required');
  process.exit(2);
}

const BRIDGE = resolveBridge({ env: process.env, home: os.homedir(), dir: __dirname, exists: fs.existsSync });
log('bridge', BRIDGE);

// --- profile and Chromium switches (before ready) ---
fs.mkdirSync(args.profile, { recursive: true, mode: 0o700 });
fs.chmodSync(args.profile, 0o700);
app.setPath('userData', args.profile);

const UA = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${process.versions.chrome.split('.')[0]}.0.0.0 Safari/537.36`;
app.userAgentFallback = UA;

const sw = (name, value) => { app.commandLine.appendSwitch(name, value); log('switch', name, value ?? ''); };
if (!args.noAutoplaySwitch) sw('autoplay-policy', 'no-user-gesture-required');
if (!args.allowThrottling) {
  sw('disable-renderer-backgrounding');
  sw('disable-background-timer-throttling');
}
if (!args.keepMediaSession) sw('disable-features', 'MediaSessionService,HardwareMediaKeyHandling');

// --- socket ---
let win = null;
let bridgeReady = false;
let quitting = false;
app.on('before-quit', () => { quitting = true; });
const sock = net.createConnection(args.socket);
const send = (obj) => {
  if (!sock.writable) return; // peer gone: dropping is fine, we are quitting
  try { sock.write(JSON.stringify(obj) + '\n', () => {}); } catch (e) { log('send failed', e.message); }
};
// The hello listener is registered first, so a queued cdm event always follows hello.
const sendEvt = (evt) => { if (sock.connecting) sock.once('connect', () => send({ t: 'evt', evt })); else send({ t: 'evt', evt }); };
sock.on('error', () => {}); // logged and handled by the quit hook below
// Never show Electron's modal error dialog: it blocks app.quit and cookie flush.
process.on('uncaughtException', (e) => {
  log('uncaught', e?.code ?? '', e?.message ?? e);
  if (e?.code === 'EPIPE') app.quit(); else app.exit(1);
});
const ok = (id) => send({ t: 'res', id, outcome: { status: 'ok', data: null } });
const applyWin = (a) => { if (!win) return; if (a === 'show') { win.show(); win.focus(); } else if (a === 'hide') win.hide(); };
const unavailable = (id) => send({ t: 'res', id, outcome: { status: 'err', error: { kind: { code: 'unavailable' }, message: 'bridge not ready' } } });

sock.on('connect', () => send({
  t: 'hello',
  proto: { major: 1, minor: 2 },
  role: 'engine',
  capabilities: ['playback', 'queue', 'api', 'window'],
  engine: `presto-engine ecs ${process.versions.electron}`,
}));
for (const ev of ['end', 'close', 'error']) {
  sock.on(ev, (e) => { // clean quit flushes cookies; hard exit so no orphan outlives presto
    log('socket', ev, e?.message ?? '');
    quitting = true; app.quit(); setTimeout(() => app.exit(0), 3000).unref();
  });
}

readline.createInterface({ input: sock }).on('line', (line) => {
  let f;
  try { f = JSON.parse(line); } catch (e) { log('bad line', e.message); return; }
  switch (f.t) {
    case 'hello':
      if (f.proto?.major !== 1) {
        log(`protocol mismatch: engine 1, presto ${f.proto?.major}.${f.proto?.minor}`);
        app.exit(2);
      }
      break;
    case 'ping': send({ t: 'pong', seq: f.seq }); break; // answered here, never via the page
    case 'cmd':
      if (f.cmd?.type === 'show_window') {
        if (!win) unavailable(f.id);
        else { applyWin(windowAction({ type: 'show_window', show: f.cmd.show === true })); ok(f.id); }
        break;
      }
    // fallthrough
    case 'req':
      if (!bridgeReady || !win) unavailable(f.id);
      else win.webContents.send('presto:in', f);
      break;
    case 'mock': log('ignoring mock frame'); break;
    default: log('unknown frame', f.t);
  }
});

// --- window ---
const trusted = (e) => win && e.sender === win.webContents && allowed(e.senderFrame?.url ?? '');
ipcMain.on('presto:ready', (e, d) => {
  if (!trusted(e)) return;
  const version = typeof d?.version === 'string' ? d.version.slice(0, 64) : 'unknown';
  const capabilities = Array.isArray(d?.capabilities) ? d.capabilities.filter((c) => typeof c === 'string').slice(0, 32) : [];
  const musickit_build = typeof d?.musickit_build === 'string' ? d.musickit_build.slice(0, 128) : null;
  bridgeReady = true;
  log('presto-diag ready ' + JSON.stringify(d?.diag));
  send({ t: 'evt', evt: { type: 'bridge_ready', version, capabilities, musickit_build } });
});
ipcMain.on('presto:out', (e, frame) => {
  if (!trusted(e)) return;
  if (frame.t === 'evt' && frame.evt?.type === 'auth') {
    log('auth', frame.evt.state);
    applyWin(windowAction({ type: 'auth', state: frame.evt.state }));
  }
  send(frame);
});

function attachDiag(wc) {
  const dbg = wc.debugger;
  dbg.attach('1.3');
  dbg.sendCommand('Network.enable');
  const hls = new Map();
  const strip = (u) => { try { const x = new URL(u); return x.origin + x.pathname; } catch { return '<bad-url>'; } };
  dbg.on('message', async (_e, method, p) => {
    try {
      if (method === 'Network.responseReceived') {
        const { url, mimeType } = p.response;
        if (p.type === 'Media') log('presto-diag media', mimeType);
        if (new URL(url).pathname.endsWith('.m3u8') || /mpegurl/i.test(mimeType)) {
          log('presto-diag hls', strip(url), mimeType);
          hls.set(p.requestId, true);
        }
      } else if (method === 'Network.loadingFinished' && hls.delete(p.requestId)) {
        const r = await dbg.sendCommand('Network.getResponseBody', { requestId: p.requestId });
        const body = r.base64Encoded ? Buffer.from(r.body, 'base64').toString() : r.body;
        for (const l of body.split('\n')) {
          if (/^#EXT-X-(STREAM-INF|MEDIA|KEY)/.test(l)) log('presto-diag m3u8', l.replace(/URI="[^"]*"/g, 'URI="<stripped>"'));
        }
      }
    } catch (e) { log('diag error', e.message); }
  });
}

app.whenReady().then(async () => {
  sendEvt({ type: 'cdm', state: 'checking', version: null, message: null });
  try {
    await withTimeout(components.whenReady(), CDM_TIMEOUT_MS);
  } catch (e) {
    const message = cdmMessage(e);
    log('cdm failed', message);
    sendEvt({ type: 'cdm', state: 'failed', version: null, message });
    return; // D-11: stay alive with no window; presto shows the error, retry is the next launch
  }
  const st = components.status();
  log('cdm', JSON.stringify(st));
  const version = st?.[components.WIDEVINE_CDM_ID]?.version;
  sendEvt({ type: 'cdm', state: 'ready', version: typeof version === 'string' ? version.slice(0, 64) : null, message: null });
  const ses = session.fromPartition('persist:presto');
  ses.setUserAgent(UA);
  ses.setPermissionRequestHandler((_wc, perm, cb) => {
    const ok = PERMISSIONS.has(perm);
    if (!ok) log('denied permission', perm);
    cb(ok);
  });
  ses.setPermissionCheckHandler((_wc, perm) => PERMISSIONS.has(perm));

  win = new BrowserWindow({
    show: args.show,
    width: 1100,
    height: 800,
    title: 'Presto engine',
    webPreferences: {
      partition: 'persist:presto',
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      plugins: true,
      backgroundThrottling: args.allowThrottling,
    },
  });
  win.on('close', (e) => { if (windowAction({ type: 'close', quitting }) === 'hide') { e.preventDefault(); win.hide(); } });
  const wc = win.webContents;
  wc.on('will-navigate', (e, url) => { if (!allowed(url)) { e.preventDefault(); log('denied navigation', safe(url)); } });
  wc.on('will-redirect', (e, url) => { if (!allowed(url)) { e.preventDefault(); log('denied redirect', safe(url)); } });
  wc.setWindowOpenHandler(({ url }) => {
    if (allowed(url)) setImmediate(() => wc.loadURL(url)); // popups load in the single window
    else log('denied window', safe(url));
    return { action: 'deny' };
  });

  // D-14: read from disk every time so edits apply on the next restart.
  const inject = () => wc.executeJavaScript(fs.readFileSync(BRIDGE, 'utf8'))
    .catch((e) => log('inject failed', e.message));
  wc.on('did-finish-load', inject);
  wc.on('did-navigate', inject);
  wc.on('did-start-navigation', (d) => { if (d.isMainFrame && !d.isSameDocument) bridgeReady = false; });

  if (args.diag) attachDiag(wc);
  win.loadURL(args.url, { userAgent: UA });
});

app.on('window-all-closed', () => {}); // close hides (D-07); quit comes from socket close
