// The only desktop features the book's page can reach: quitting and full screen.
// The runtime checks for `window.tomeDesktop` and shows these controls only when it exists.

const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('tomeDesktop', {
  quit: () => ipcRenderer.send('tome:quit'),
  isFullscreen: () => ipcRenderer.invoke('tome:is-fullscreen'),
  setFullscreen: (on) => ipcRenderer.send('tome:set-fullscreen', Boolean(on)),
  /** Calls back whenever full screen changes, including from F11; returns an unsubscribe function. */
  onFullscreenChange: (callback) => {
    const listener = (_event, on) => callback(on)
    ipcRenderer.on('tome:fullscreen', listener)
    return () => ipcRenderer.removeListener('tome:fullscreen', listener)
  },
})
