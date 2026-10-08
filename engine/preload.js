const { contextBridge, ipcRenderer } = require('electron');
let handler = null;
ipcRenderer.on('presto:in', (_e, frame) => { if (handler) handler(frame); });
contextBridge.exposeInMainWorld('__presto', {
  ready: (diag) => ipcRenderer.send('presto:ready', diag),
  emit: (evt) => ipcRenderer.send('presto:out', { t: 'evt', evt }),
  reply: (id, outcome) => ipcRenderer.send('presto:out', { t: 'res', id, outcome }),
  onFrame: (cb) => { handler = cb; },
});
