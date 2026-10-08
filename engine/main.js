// presto-engine: castlabs ECS hosting music.apple.com, speaking presto-ipc over a Unix socket.
// All logging goes to stderr; stdout stays silent.
const fs = require('fs');
const net = require('net');
const path = require('path');
const readline = require('readline');
const { app, BrowserWindow, ipcMain, session, components } = require('electron');

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
const sock = net.createConnection(args.socket);
const send = (obj) => { if (!sock.destroyed) sock.write(JSON.stringify(obj) + '\n'); };
const unavailable = (id) => send({ t: 'res', id, outcome: { status: 'err', error: { kind: { code: 'unavailable' }, message: 'bridge not ready' } } });

sock.on('connect', () => send({
  t: 'hello',
  proto: { major: 1, minor: 0 },
  role: 'engine',
  capabilities: ['playback', 'queue', 'api'],
  engine: `presto-engine ecs ${process.versions.electron}`,
}));
for (const ev of ['end', 'close', 'error']) {
  sock.on(ev, (e) => { log('socket', ev, e?.message ?? ''); app.quit(); }); // clean quit flushes cookies
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
    case 'req':
      if (!bridgeReady || !win) unavailable(f.id);
      else win.webContents.send('presto:in', f);
      break;
    case 'mock': log('ignoring mock frame'); break;
    default: log('unknown frame', f.t);
  }
});

// --- window ---
ipcMain.on('presto:ready', (e, diag) => {
  if (!win || e.sender !== win.webContents) return;
  bridgeReady = true;
  log('presto-diag ready ' + JSON.stringify(diag));
});
ipcMain.on('presto:out', (e, frame) => {
  if (!win || e.sender !== win.webContents) return;
  if (frame.t === 'evt' && frame.evt?.type === 'auth') {
    log('auth', frame.evt.state);
    if (frame.evt.state === 'signed_out') win.show(); // sign-in needs a visible window
    else if (frame.evt.state === 'signed_in') win.hide(); // D-11
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
  await components.whenReady();
  log('cdm', JSON.stringify(components.status()));
  session.fromPartition('persist:presto').setUserAgent(UA);

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
  const wc = win.webContents;

  // D-08: read from disk every time so edits apply on the next restart.
  const inject = () => wc.executeJavaScript(fs.readFileSync(path.join(__dirname, 'bridge.js'), 'utf8'))
    .catch((e) => log('inject failed', e.message));
  wc.on('did-finish-load', inject);
  wc.on('did-navigate', inject);
  wc.on('did-start-navigation', (d) => { if (d.isMainFrame && !d.isSameDocument) bridgeReady = false; });

  if (args.diag) attachDiag(wc);
  win.loadURL(args.url, { userAgent: UA });
});

app.on('window-all-closed', () => app.quit());
