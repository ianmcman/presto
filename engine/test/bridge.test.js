const test = require('node:test');
const assert = require('node:assert/strict');
const { createBridge, mapState, mapItem, readyInfo, BRIDGE_VERSION } = require('../bridge.js');

const all = []; // every payload, for the token check

function setup(over = {}) {
  const MusicKit = {
    PlaybackStates: { none: 0, loading: 1, playing: 2, paused: 3, stopped: 4, ended: 5, seeking: 6, waiting: 8, stalled: 9, completed: 10 },
    PlayerRepeatMode: { none: 0, one: 1, all: 2 },
    PlayerShuffleMode: { off: 0, songs: 1 },
    version: '3.test',
  };
  const calls = [];
  const handlers = {};
  const mk = {
    calls, handlers,
    play: () => calls.push(['play']),
    pause: () => calls.push(['pause']),
    seekToTime: (s) => calls.push(['seek', s]),
    skipToNextItem: () => calls.push(['next']),
    skipToPreviousItem: () => calls.push(['prev']),
    setQueue: (o) => calls.push(['setQueue', o]),
    changeToMediaAtIndex: (i) => calls.push(['idx', i]),
    addEventListener: (n, f) => { handlers[n] = f; },
    isAuthorized: true, playbackState: 0, currentPlaybackTime: 0, currentPlaybackDuration: 0,
    isPlaying: false, nowPlayingItem: null, queue: { items: [], position: 0 }, volume: 1,
    api: { music: async () => ({ data: { data: [{ id: 'p.1' }], next: '/n' } }) },
    ...over,
  };
  const out = { events: [], replies: [], readies: [] };
  const presto = {
    emit: (e) => { out.events.push(e); all.push(e); },
    reply: (id, o) => { out.replies.push([id, o]); all.push(o); },
    ready: (d) => { out.readies.push(d); all.push(d); },
  };
  let tick;
  const b = createBridge({ MusicKit, mk, presto, setInterval: (f) => { tick = f; } });
  return { MusicKit, mk, out, b, tick: () => tick() };
}

const cmd = (id, c) => ({ t: 'cmd', id, cmd: c });
const req = (id, r) => ({ t: 'req', id, req: { method: 'get', path: '/v1/x', query: {}, body: null, ...r } });
const OK = { status: 'ok', data: null };

test('mapState', () => {
  const { MusicKit } = setup();
  const m = (n) => mapState(MusicKit, n);
  assert.equal(m(2), 'playing');
  assert.equal(m(3), 'paused');
  for (const n of [1, 6, 8, 9, 99]) assert.equal(m(n), 'loading');
  for (const n of [5, 10]) assert.equal(m(n), 'ended');
  for (const n of [0, 4]) assert.equal(m(n), 'stopped');
});

test('mapItem', () => {
  assert.deepEqual(
    mapItem({ id: 123, attributes: { name: 'A', artistName: 'B', albumName: 'C', durationInMillis: 241000.4, artwork: { url: 'https://x/{w}x{h}.jpg' } } }),
    { id: '123', title: 'A', artist: 'B', album: 'C', duration_ms: 241000, artwork_url: 'https://x/600x600.jpg', playable: true });
  assert.equal(mapItem(null), null);
});

test('play/pause/next/prev reply ok and bump seq', async () => {
  const { b, mk, out } = setup();
  await b.handle(cmd(1, { type: 'play' }));
  assert.deepEqual(out.replies[0], [1, OK]);
  assert.equal(mk.calls.filter((c) => c[0] === 'play').length, 1);
  await b.handle(cmd(2, { type: 'pause' }));
  await b.handle(cmd(3, { type: 'next' }));
  await b.handle(cmd(4, { type: 'prev' }));
  b.start();
  mk.handlers.playbackStateDidChange({ state: 2 });
  assert.equal(out.events.find((e) => e.type === 'playback_state').seq, 4);
});

test('seek, volume, repeat', async () => {
  const { b, mk } = setup();
  await b.handle(cmd(1, { type: 'seek', ms: 120000 }));
  assert.deepEqual(mk.calls[0], ['seek', 120]);
  await b.handle(cmd(2, { type: 'set_volume', volume: 0.4 }));
  assert.equal(mk.volume, 0.4);
  mk.repeatMode = 9;
  await b.handle(cmd(3, { type: 'set_repeat', mode: 'off' }));
  assert.equal(mk.repeatMode, 0);
  await b.handle(cmd(4, { type: 'set_repeat', mode: 'all' }));
  assert.equal(mk.repeatMode, 2);
  await b.handle(cmd(5, { type: 'set_shuffle', on: true }));
  assert.equal(mk.shuffleMode, 1);
});

test('set_queue', async () => {
  const { b, mk } = setup();
  await b.handle(cmd(1, { type: 'set_queue', ids: ['1', '2'], start: 0 }));
  assert.deepEqual(mk.calls, [['setQueue', { songs: ['1', '2'], startPlaying: true }]]);
  await b.handle(cmd(2, { type: 'set_queue', ids: ['1', '2'], start: 1 }));
  assert.deepEqual(mk.calls.at(-1), ['idx', 1]);
});

test('set_queue station', async () => {
  const { b, mk } = setup();
  await b.handle(cmd(1, { type: 'set_queue', ids: ['ra.123'], start: 0 }));
  assert.deepEqual(mk.calls, [['setQueue', { station: 'ra.123', startPlaying: true }]]);
});

test('set_queue play flag', async () => {
  const { b, mk } = setup();
  await b.handle(cmd(1, { type: 'set_queue', ids: ['1'], start: 0, play: false }));
  assert.equal(mk.calls[0][1].startPlaying, false);
});

test('readyInfo', () => {
  const { MusicKit } = setup();
  const info = readyInfo(MusicKit);
  all.push(info);
  assert.deepEqual(info, { version: BRIDGE_VERSION, capabilities: ['playback', 'queue', 'api'], musickit_build: '3.test' });
  assert.equal(readyInfo({}).musickit_build, null);
});

test('unknown cmd is internal', async () => {
  const { b, out } = setup();
  await b.handle(cmd(1, { type: 'bogus' }));
  assert.equal(out.replies[0][1].status, 'err');
  assert.deepEqual(out.replies[0][1].error.kind, { code: 'internal' });
});

test('req get ok', async () => {
  const seen = [];
  const { b, out } = setup({ api: { music: async (p, q) => { seen.push([p, q]); return { data: { data: [{ id: 'p.1' }], next: '/n' } }; } } });
  await b.handle(req(7, { path: '/v1/me/library/playlists', query: { limit: '25' } }));
  assert.deepEqual(seen, [['/v1/me/library/playlists', { limit: '25' }]]);
  assert.deepEqual(out.replies[0], [7, { status: 'ok', data: { data: [{ id: 'p.1' }], next: '/n' } }]);
});

test('req errors', async () => {
  const cases = [
    [{ status: 401 }, { code: 'auth_expired' }],
    [{ status: 403 }, { code: 'auth_expired' }],
    [{ status: 429 }, { code: 'rate_limited', retry_after_ms: null }],
    [{ status: 404 }, { code: 'not_found' }],
    [{ status: 500 }, { code: 'upstream', status: 500 }],
    [new Error('x'), { code: 'internal' }],
  ];
  for (const [thrown, kind] of cases) {
    const { b, out } = setup({ api: { music: async () => { throw thrown; } } });
    await b.handle(req(1, {}));
    assert.deepEqual(out.replies[0][1].error.kind, kind);
  }
});

test('req post rejected', async () => {
  const { b, out } = setup();
  await b.handle(req(1, { method: 'post' }));
  assert.deepEqual(out.replies[0][1].error.kind, { code: 'internal' });
});

test('req too large', async () => {
  const { b, out } = setup({ api: { music: async () => ({ data: 'x'.repeat(4_000_001) }) } });
  await b.handle(req(1, {}));
  assert.equal(out.replies[0][1].status, 'err');
});

test('start emits auth and events', () => {
  const { b, mk, out } = setup();
  b.start();
  assert.deepEqual(out.events[0], { type: 'auth', state: 'signed_in' });
  mk.handlers.playbackStateDidChange({ state: 2 });
  assert.deepEqual(out.events.find((e) => e.type === 'playback_state'), { type: 'playback_state', state: 'playing', seq: 0 });
  mk.isAuthorized = false;
  mk.handlers.authorizationStatusDidChange();
  assert.deepEqual(out.events.at(-1), { type: 'auth', state: 'signed_out' });
  mk.handlers.queueItemsDidChange();
  mk.handlers.queueItemsDidChange();
  const q = out.events.filter((e) => e.type === 'queue_changed');
  assert.deepEqual(q.map((e) => e.rev), [1, 2]);
});

test('progress tick', () => {
  const { b, mk, out, tick } = setup();
  b.start();
  const n = out.events.length;
  tick();
  assert.equal(out.events.length, n);
  mk.isPlaying = true; mk.currentPlaybackTime = 61.25; mk.currentPlaybackDuration = 241;
  tick();
  assert.deepEqual(out.events.at(-1), { type: 'progress', position_ms: 61250, duration_ms: 241000, seq: 0 });
});

test('diag drops credential-like member names', () => {
  class K { musicUserToken() {} play() {} }
  const { b } = setup();
  const { b: b2 } = (() => {
    const s = setup();
    return { b: createBridge({ MusicKit: s.MusicKit, mk: new K(), presto: { emit() {}, reply() {}, ready() {} }, setInterval() {} }) };
  })();
  all.push(b.diag());
  assert.deepEqual(b2.diag().mk_members, ['constructor', 'play']);
});

test('no payload mentions a token', () => {
  assert.ok(all.length > 10);
  assert.doesNotMatch(JSON.stringify(all), /token/i);
});
