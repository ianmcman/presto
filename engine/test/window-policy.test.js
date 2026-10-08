const test = require('node:test');
const assert = require('node:assert/strict');
const { windowAction } = require('../window-policy.js');

test('auth', () => {
  assert.equal(windowAction({ type: 'auth', state: 'signed_in' }), 'hide');
  for (const state of ['signed_out', 'expired', 'signing_in']) assert.equal(windowAction({ type: 'auth', state }), null);
});

test('show_window', () => {
  assert.equal(windowAction({ type: 'show_window', show: true }), 'show');
  assert.equal(windowAction({ type: 'show_window', show: false }), 'hide');
});

test('close', () => {
  assert.equal(windowAction({ type: 'close', quitting: false }), 'hide');
  assert.equal(windowAction({ type: 'close', quitting: true }), 'close');
});
