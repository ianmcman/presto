const test = require('node:test');
const assert = require('node:assert/strict');
const { resolveBridge } = require('../bridge-path.js');

test('XDG_CONFIG_HOME override', () => {
  const r = resolveBridge({ env: { XDG_CONFIG_HOME: '/x' }, home: '/h', dir: '/d', exists: (p) => p === '/x/presto/bridge.js' });
  assert.equal(r, '/x/presto/bridge.js');
});

test('home .config override', () => {
  const r = resolveBridge({ env: {}, home: '/h', dir: '/d', exists: (p) => p === '/h/.config/presto/bridge.js' });
  assert.equal(r, '/h/.config/presto/bridge.js');
});

test('falls back to bundled', () => {
  assert.equal(resolveBridge({ env: {}, home: '/h', dir: '/d', exists: () => false }), '/d/bridge.js');
});
