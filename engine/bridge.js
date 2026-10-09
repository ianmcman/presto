// Injected into music.apple.com by main.js (D-08) and loaded as a module by tests.
// Maps MusicKit to presto-ipc JSON. Never reads credentials.
(function () {
  'use strict';

  const MAX_JSON = 4_000_000; // presto-ipc max line is 4 MiB
  const BRIDGE_VERSION = '1.1.0';
  const CAPABILITIES = ['playback', 'queue', 'api'];

  function readyInfo(MusicKit) {
    return { version: BRIDGE_VERSION, capabilities: CAPABILITIES.slice(), musickit_build: MusicKit.version ?? null };
  }

  function mapState(MusicKit, n) {
    const states = MusicKit.PlaybackStates || {};
    const name = Object.keys(states).find((k) => states[k] === n);
    switch (name) {
      case 'playing': return 'playing';
      case 'paused': return 'paused';
      case 'ended':
      case 'completed': return 'ended';
      case 'none':
      case 'stopped': return 'stopped';
      default: return 'loading';
    }
  }

  function mapItem(mi) {
    if (!mi) return null;
    const a = mi.attributes ?? {};
    const url = a.artwork?.url;
    return {
      id: String(mi.id),
      title: a.name ?? mi.title ?? '',
      artist: a.artistName ?? '',
      album: a.albumName ?? '',
      duration_ms: Math.round(a.durationInMillis ?? (mi.playbackDuration ?? 0)),
      artwork_url: url ? url.replace('{w}', '600').replace('{h}', '600') : null,
      playable: mi.isPlayable !== false,
    };
  }

  function mapApiError(e) {
    const status = e?.status ?? e?.response?.status ?? e?.httpStatusCode;
    let kind;
    if (status === 401 || status === 403) kind = { code: 'auth_expired' };
    else if (status === 429) kind = { code: 'rate_limited', retry_after_ms: null };
    else if (status === 404) kind = { code: 'not_found' };
    else if (Number.isInteger(status)) kind = { code: 'upstream', status };
    else kind = { code: 'internal' };
    return { kind, message: String(e?.message ?? e).slice(0, 200) };
  }

  const ok = { status: 'ok', data: null };
  const err = (message) => ({ status: 'err', error: { kind: { code: 'internal' }, message } });

  function createBridge({ MusicKit, mk, presto, setInterval: si }) {
    let seq = 0;
    let rev = 0;
    const emit = (evt) => presto.emit(evt);
    const authEvt = () => emit({ type: 'auth', state: mk.isAuthorized ? 'signed_in' : 'signed_out' });
    const progress = () => emit({
      type: 'progress',
      position_ms: Math.round((mk.currentPlaybackTime ?? 0) * 1000),
      duration_ms: Math.round((mk.currentPlaybackDuration ?? 0) * 1000),
      seq,
    });

    async function runCmd(cmd) {
      switch (cmd.type) {
        case 'play': seq++; await mk.play(); break;
        case 'pause': seq++; await mk.pause(); break;
        case 'seek': seq++; await mk.seekToTime(cmd.ms / 1000); break;
        case 'next': seq++; await mk.skipToNextItem(); break;
        case 'prev': seq++; await mk.skipToPreviousItem(); break;
        case 'set_volume': mk.volume = cmd.volume; break;
        case 'set_shuffle':
          mk.shuffleMode = cmd.on ? (MusicKit.PlayerShuffleMode?.songs ?? 1) : (MusicKit.PlayerShuffleMode?.off ?? 0);
          break;
        case 'set_repeat':
          mk.repeatMode = MusicKit.PlayerRepeatMode?.[cmd.mode === 'off' ? 'none' : cmd.mode]
            ?? { off: 0, one: 1, all: 2 }[cmd.mode];
          break;
        case 'set_queue':
          seq++;
          // Stations are not song lists; a lone "ra.*" id is a station (catalog and personal).
          await mk.setQueue({
            ...(cmd.ids.length === 1 && /^ra\./.test(cmd.ids[0]) ? { station: cmd.ids[0] } : { songs: cmd.ids }),
            startPlaying: cmd.play !== false,
          });
          if (cmd.start > 0) await mk.changeToMediaAtIndex(cmd.start);
          break;
        default: throw new Error('unknown command ' + cmd.type);
      }
    }

    async function handle(frame) {
      if (frame.t === 'cmd') {
        try {
          await runCmd(frame.cmd);
          presto.reply(frame.id, ok);
        } catch (e) {
          const o = mapApiError(e);
          const hasStatus = (e?.status ?? e?.response?.status ?? e?.httpStatusCode) !== undefined;
          presto.reply(frame.id, { status: 'err', error: hasStatus ? o : { kind: { code: 'internal' }, message: o.message } });
        }
      } else if (frame.t === 'req') {
        const req = frame.req;
        if (req.method !== 'get') return presto.reply(frame.id, err('spike engine supports GET only'));
        try {
          const res = await mk.api.music(req.path, req.query ?? {});
          const json = JSON.stringify(res?.data ?? res);
          if (json.length > MAX_JSON) return presto.reply(frame.id, err('response too large; lower limit'));
          presto.reply(frame.id, { status: 'ok', data: JSON.parse(json) });
        } catch (e) {
          presto.reply(frame.id, { status: 'err', error: mapApiError(e) });
        }
      }
    }

    function start() {
      authEvt();
      mk.addEventListener('playbackStateDidChange', (e) => {
        emit({ type: 'playback_state', state: mapState(MusicKit, e?.state ?? mk.playbackState), seq });
        progress();
      });
      const track = () => emit({ type: 'track_changed', item: mapItem(mk.nowPlayingItem) });
      mk.addEventListener('mediaItemDidChange', track);
      mk.addEventListener('nowPlayingItemDidChange', track);
      const queue = () => {
        rev++;
        const pos = mk.queue?.position;
        emit({
          type: 'queue_changed',
          rev,
          items: (mk.queue?.items ?? []).map(mapItem),
          index: pos >= 0 ? pos : null,
        });
      };
      mk.addEventListener('queueItemsDidChange', queue);
      mk.addEventListener('queuePositionDidChange', queue);
      mk.addEventListener('playbackVolumeDidChange', () => emit({ type: 'volume', volume: mk.volume }));
      mk.addEventListener('authorizationStatusDidChange', authEvt);
      si(() => { if (mk.isPlaying) progress(); }, 500);
    }

    // Member names only, never values. Names mentioning credentials are dropped.
    const names = (o) => Object.getOwnPropertyNames(Object.getPrototypeOf(o))
      .filter((n) => !/token/i.test(n));

    function diag() {
      let build = null;
      if (typeof document !== 'undefined') {
        const s = Array.from(document.scripts).map((x) => x.src)
          .find((u) => u && /musickit|index/i.test(u));
        build = s ? s.split('?')[0] : null;
      }
      return {
        bridge: 1,
        musickit_version: MusicKit.version ?? null,
        build,
        playback_states: MusicKit.PlaybackStates,
        mk_members: names(mk),
        queue_members: names(mk.queue ?? {}),
        api_music_arity: mk.api?.music?.length ?? null,
        has_media_session: typeof navigator !== 'undefined' && 'mediaSession' in navigator,
        ua: typeof navigator !== 'undefined' ? navigator.userAgent : null,
      };
    }

    return { handle, start, diag };
  }

  function install(win) {
    if (win.__prestoBridge) return;
    win.__prestoBridge = true;
    const t = setInterval(() => {
      if (!win.MusicKit) return;
      let mk;
      try { mk = win.MusicKit.getInstance(); } catch { return; }
      if (!mk) return;
      clearInterval(t);
      const b = createBridge({ MusicKit: win.MusicKit, mk, presto: win.__presto, setInterval });
      win.__presto.onFrame((f) => b.handle(f));
      win.__presto.ready({ ...readyInfo(win.MusicKit), diag: b.diag() });
      b.start();
    }, 250);
  }

  if (typeof module === 'object' && module.exports) {
    module.exports = { createBridge, mapState, mapItem, mapApiError, install, BRIDGE_VERSION, CAPABILITIES, readyInfo };
  } else {
    install(window);
  }
})();
