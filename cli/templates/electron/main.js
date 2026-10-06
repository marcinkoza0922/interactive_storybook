// The desktop shell for a book built with tome. It shows the book's web build in a window,
// served from app://book/: a custom secure origin, so fetch(), localStorage and media seeking
// behave as they do on the web (they don't from file://).

const { app, BrowserWindow, Menu, protocol, shell } = require('electron')
const fs = require('node:fs')
const path = require('node:path')
const { Readable } = require('node:stream')

const { tome: book } = require('./package.json')
const WEB_ROOT = path.join(__dirname, 'web')

// Each book keeps its own saved progress, separate from other books built with tome.
app.setPath('userData', path.join(app.getPath('appData'), 'tome-books', book.id))

protocol.registerSchemesAsPrivileged([
  { scheme: 'app', privileges: { standard: true, secure: true, supportFetchAPI: true, stream: true } },
])

const TYPES = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.gif': 'image/gif',
  '.webp': 'image/webp', '.avif': 'image/avif', '.ogg': 'audio/ogg', '.opus': 'audio/ogg', '.mp3': 'audio/mpeg',
  '.m4a': 'audio/mp4', '.aac': 'audio/mp4', '.wav': 'audio/wav', '.flac': 'audio/flac', '.webm': 'video/webm',
  '.mp4': 'video/mp4', '.ogv': 'video/ogg', '.woff2': 'font/woff2', '.woff': 'font/woff', '.ttf': 'font/ttf', '.otf': 'font/otf',
}

/** Serve a file from the web build, honouring byte ranges so audio and video can seek and loop. */
async function serve(request) {
  const { pathname } = new URL(request.url)
  const file = path.join(WEB_ROOT, decodeURIComponent(pathname === '/' ? '/index.html' : pathname))
  if (!file.startsWith(WEB_ROOT + path.sep)) return new Response('Not found', { status: 404 })

  let size
  try {
    const stat = await fs.promises.stat(file)
    if (!stat.isFile()) throw new Error('not a file')
    size = stat.size
  } catch {
    return new Response('Not found', { status: 404 })
  }

  const headers = { 'Content-Type': TYPES[path.extname(file).toLowerCase()] ?? 'application/octet-stream', 'Accept-Ranges': 'bytes' }
  const range = /^bytes=(\d*)-(\d*)$/.exec(request.headers.get('Range') ?? '')
  if (range && size > 0) {
    let start = range[1] === '' ? Math.max(size - Number(range[2]), 0) : Number(range[1])
    let end = range[1] !== '' && range[2] !== '' ? Math.min(Number(range[2]), size - 1) : size - 1
    if (start <= end) {
      const body = Readable.toWeb(fs.createReadStream(file, { start, end }))
      return new Response(body, {
        status: 206,
        headers: { ...headers, 'Content-Range': `bytes ${start}-${end}/${size}`, 'Content-Length': String(end - start + 1) },
      })
    }
  }
  return new Response(Readable.toWeb(fs.createReadStream(file)), { headers: { ...headers, 'Content-Length': String(size) } })
}

function createWindow() {
  const window = new BrowserWindow({
    width: 1280,
    height: 860,
    minWidth: 360,
    minHeight: 480,
    title: book.title,
    backgroundColor: book.background,
    autoHideMenuBar: true,
    show: false,
    webPreferences: {
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false,
      spellcheck: false,
      // A desktop book may start its music from a controller press, which browsers don't count
      // as the gesture that unlocks audio.
      autoplayPolicy: 'no-user-gesture-required',
    },
  })
  window.once('ready-to-show', () => window.show())

  // The book never navigates away; links to the web open in the reader's browser.
  const openOutside = (url) => /^https?:\/\//.test(url) && shell.openExternal(url)
  window.webContents.setWindowOpenHandler(({ url }) => {
    openOutside(url)
    return { action: 'deny' }
  })
  window.webContents.on('will-navigate', (event, url) => {
    if (url.startsWith('app://book/')) return
    event.preventDefault()
    openOutside(url)
  })

  // F11 or Alt+Enter toggles full screen; there's no menu bar to do it from.
  window.webContents.on('before-input-event', (event, input) => {
    if (input.type === 'keyDown' && (input.key === 'F11' || (input.alt && input.key === 'Enter'))) {
      window.setFullScreen(!window.isFullScreen())
      event.preventDefault()
    }
  })

  window.loadURL('app://book/index.html')
  return window
}

if (!app.requestSingleInstanceLock()) {
  app.quit()
} else {
  let window = null
  app.on('second-instance', () => {
    if (!window) return
    if (window.isMinimized()) window.restore()
    window.focus()
  })
  app.whenReady().then(() => {
    Menu.setApplicationMenu(null)
    protocol.handle('app', serve)
    window = createWindow()
  })
  app.on('window-all-closed', () => app.quit())
}
