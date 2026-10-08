const test = require('node:test');
const assert = require('node:assert');
const { allowed, safe } = require('../guard');

test('allowlist', () => {
  for (const u of ['https://music.apple.com/us', 'https://apple.com/', 'https://is1-ssl.mzstatic.com/a.jpg', 'https://www.icloud.com/', 'about:blank']) assert.ok(allowed(u), u);
  for (const u of ['http://music.apple.com/', 'https://evilapple.com/', 'https://apple.com.evil.io/', 'file:///etc/passwd', 'javascript:1', '', 'nonsense']) assert.ok(!allowed(u), u);
});
test('safe strips query', () => assert.strictEqual(safe('https://a.apple.com/x?token=1#f'), 'https://a.apple.com/x'));
