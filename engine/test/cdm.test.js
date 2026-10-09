const test = require('node:test');
const assert = require('node:assert/strict');
const { CDM_TIMEOUT_MS, cdmMessage, withTimeout } = require('../cdm.js');

test('cdmMessage formats component errors', () => {
  const d = (id, status) => ({ detail: { id, status } });
  assert.equal(cdmMessage({ errors: [d('oimompecagnajdejgnnjijobebaeigek', 'error')] }), 'oimompecagnajdejgnnjijobebaeigek: error');
  assert.equal(cdmMessage({ errors: [d('a', 'x'), d('b', 'y')] }), 'a: x; b: y');
});

test('cdmMessage falls back', () => {
  assert.equal(cdmMessage(new Error('boom')), 'boom');
  assert.equal(cdmMessage(undefined), 'unknown error');
  assert.equal(cdmMessage(new Error('x'.repeat(500))).length, 300);
});

test('withTimeout', async () => {
  assert.equal(await withTimeout(Promise.resolve(1), 50), 1);
  await assert.rejects(withTimeout(new Promise(() => {}), 20), { message: 'timed out after 0.02 s waiting for the Widevine CDM' });
  assert.equal(CDM_TIMEOUT_MS, 120000);
});
