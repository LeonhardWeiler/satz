import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import CanvasKitInit from 'canvaskit-wasm'
import canvaskitWasm from 'canvaskit-wasm/bin/canvaskit.wasm?url'
import init, { Engine } from './engine/engine'
import { App } from './App'
import { Editor } from './editor'
import { download, findGrantedFonts, keepUnreadable, stored, storedFonts } from './file'
import './index.css'

if (import.meta.env.PROD) navigator.serviceWorker?.register(`${import.meta.env.BASE_URL}sw.js`)

const root = createRoot(document.getElementById('root')!)
addEventListener('pointerdown', () => document.documentElement.toggleAttribute('data-pointer', true), true)
addEventListener('keydown', () => document.documentElement.removeAttribute('data-pointer'), true)

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
    const [ck, , saved, fonts, alone] = await Promise.all([
      CanvasKitInit({ locateFile: () => canvaskitWasm }),
      init(),
      stored().catch(() => undefined),
      storedFonts().catch(() => []),
      new Promise<boolean>((got) =>
        navigator.locks
          ? navigator.locks.request('satz', { ifAvailable: true }, (lock) => {
              got(!!lock)
              return lock && new Promise(() => {})
            })
          : got(true),
      ),
    ])
    const engine = saved ? new Engine() : Engine.sample()
    for (const bytes of fonts) engine.addFont(bytes)
    const editor = new Editor(engine)
    let first = !saved
    let trap = false
    try {
      if (saved) editor.load(saved.bytes, { name: saved.name, handle: saved.handle }, saved.dirty, saved.updates)
    } catch (e) {
      if (!(e instanceof WebAssembly.RuntimeError)) editor.load(new Engine().save())
      await keepUnreadable(saved!)
      download(saved!.bytes, saved!.name, 'application/x-satz')
      editor.status = `Could not restore ${saved!.name}: ${(e as Error).message}. Satz downloaded it and started a new document.`
      first = true
      trap = e instanceof WebAssembly.RuntimeError
    }
    const stop = () => {
      editor.autosaves = false
      notice('Satz stopped', 'The engine failed. Reload the page to go on from the last autosave.', true)
    }
    if (trap) notice('Satz stopped', `Restoring ${saved!.name} failed. Satz downloaded it; reload the page to start a new document.`, true)
    else
      root.render(
        <StrictMode>
          <App ck={ck} editor={editor} first={first} />
        </StrictMode>,
      )
    if (!alone) {
      editor.autosaves = false
      editor.fail('Satz is open in another tab. This tab does not autosave; save the file with Ctrl+S.')
    }
    const trapped = (e: unknown) => e instanceof WebAssembly.RuntimeError && stop()
    window.addEventListener('error', (e) => trapped(e.error))
    window.addEventListener('unhandledrejection', (e) => trapped(e.reason))
    findGrantedFonts(editor)
  } catch (e) {
    console.error(e)
    notice('Satz could not start', 'Loading the engine failed. Check your connection and reload the page.', true)
  }
}
