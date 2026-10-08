const path = require('path');

// D-14: a user copy in $XDG_CONFIG_HOME/presto/bridge.js overrides the bundled bridge.
function resolveBridge({ env, home, dir, exists }) {
  const base = env.XDG_CONFIG_HOME || path.join(home, '.config');
  const user = path.join(base, 'presto', 'bridge.js');
  return exists(user) ? user : path.join(dir, 'bridge.js');
}

module.exports = { resolveBridge };
