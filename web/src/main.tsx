import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import CanvasKitInit from '../canvaskit/canvaskit.js'
import canvaskitWasm from '../canvaskit/canvaskit.wasm?url'
import init, { Engine } from './engine/engine'
import { App } from './App'
import { restorePanels } from './controls'
import { Editor } from './editor'
import { findGranted, openProject, show, storedFonts, warm } from './file'
import { followBrowser } from './settings'
import './index.css'

followBrowser()
restorePanels()
const root = createRoot(document.getElementById('root')!)
addEventListener('pointerdown', () => document.documentElement.toggleAttribute('data-pointer', true), true)
addEventListener('keydown', (e) => (e.key === 'Tab' || e.key === 'F6') && document.documentElement.removeAttribute('data-pointer'), true)

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
    const [ck, , fonts, alone] = await Promise.all([
      CanvasKitInit({ locateFile: () => canvaskitWasm }),
      init(),
      storedFonts().catch(() => []),
      new Promise<boolean>((got) => {
        if (!navigator.locks) return got(true)
        void navigator.locks.request('satz', { ifAvailable: true }, (lock) => {
          got(!!lock)
          return lock && new Promise(() => {})
        })
      }),
    ])
    const engine = new Engine()
    for (const bytes of fonts) engine.addFont(bytes)
    const editor = new Editor(engine)
    const stop = () => {
      notice('Satz stopped', 'The engine failed. Reload the page to go on from the last autosave.', true)
      editor.set({ autosaves: false })
    }
    Engine.onPanic(() => queueMicrotask(stop))
    if (!alone) {
      editor.set({ autosaves: false })
      editor.fail('Satz is open in another tab. This tab does not autosave; save the file with Ctrl+S.')
      void navigator.locks.request('satz', () => {
        editor.set({ autosaves: true })
        return new Promise(() => {})
      })
    }
    const id = location.hash.slice(1)
    if (id) await openProject(editor, id)
    if (!editor.project) show(editor, null)
    root.render(
      <StrictMode>
        <App ck={ck} editor={editor} />
      </StrictMode>,
    )
    warm()
    if (import.meta.env.PROD) navigator.serviceWorker?.register(`${import.meta.env.BASE_URL}sw.js`).catch(console.error)
    void findGranted(editor)
  } catch (e) {
    console.error(e)
    notice('Satz could not start', 'Loading the engine failed. Check your connection and reload the page.', true)
  }
}
