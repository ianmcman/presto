// Rust owns showing (show_window). The engine only hides on signed_in (D-08) and turns close into hide (D-07).
function windowAction(ev) {
  switch (ev.type) {
    case 'auth': return ev.state === 'signed_in' ? 'hide' : null;
    case 'show_window': return ev.show ? 'show' : 'hide';
    case 'close': return ev.quitting ? 'close' : 'hide';
    default: return null;
  }
}

module.exports = { windowAction };
