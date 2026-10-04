import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import CanvasKitInit from 'canvaskit-wasm'
import canvaskitWasm from 'canvaskit-wasm/bin/canvaskit.wasm?url'
import init, { Engine } from './engine/engine'
import { App } from './App'
import { Editor } from './editor'
import { download, keepUnreadable, stored, storedFonts } from './file'
import './index.css'

if (import.meta.env.PROD) navigator.serviceWorker?.register(`${import.meta.env.BASE_URL}sw.js`)

const root = createRoot(document.getElementById('root')!)

const notice = (title: string, text: string, reload = false) =>
  root.render(
    <main className="notice" role="alert">
      <h1>{title}</h1>
      <p>{text}</p>
      {reload && <button onClick={() => location.reload()}>Reload</button>}
    </main>,
  )

if (!document.createElement('canvas').getContext('webgl2')) {
  notice(
    'WebGL 2 is not available',
    'Satz draws the page with WebGL 2. Turn on hardware acceleration in your browser settings, or open Satz in a current Chrome or Firefox.',
  )
} else {
  try {
    const [ck, , saved, fonts] = await Promise.all([
      CanvasKitInit({ locateFile: () => canvaskitWasm }),
      init(),
      stored().catch(() => undefined),
      storedFonts().catch(() => []),
    ])
    const engine = saved ? new Engine() : Engine.sample()
    for (const bytes of fonts) engine.addFont(bytes)
    const editor = new Editor(engine)
    let first = !saved
    try {
      if (saved) editor.load(saved.bytes, { name: saved.name, handle: saved.handle }, saved.dirty)
    } catch (e) {
      await keepUnreadable(saved!)
      download(saved!.bytes, saved!.name, 'application/x-satz')
      editor.status = `Could not restore ${saved!.name}: ${(e as Error).message}. Satz downloaded it and started a new document.`
      first = true
    }
    root.render(
      <StrictMode>
        <App ck={ck} editor={editor} first={first} />
      </StrictMode>,
    )
    const trapped = (e: unknown) => {
      if (!(e instanceof WebAssembly.RuntimeError)) return
      editor.broken = true
      notice('Satz stopped', 'The engine failed. Reload the page to go on from the last autosave.', true)
    }
    window.addEventListener('error', (e) => trapped(e.error))
    window.addEventListener('unhandledrejection', (e) => trapped(e.reason))
  } catch (e) {
    console.error(e)
    notice('Satz could not start', 'Loading the engine failed. Check your connection and reload the page.', true)
  }
}
