import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import CanvasKitInit from 'canvaskit-wasm'
import canvaskitWasm from 'canvaskit-wasm/bin/canvaskit.wasm?url'
import init, { Engine } from './engine/engine'
import { App } from './App'
import { Editor } from './editor'
import { stored, storedFonts } from './file'
import './index.css'

const root = createRoot(document.getElementById('root')!)

const notice = (title: string, text: string) =>
  root.render(
    <main className="notice" role="alert">
      <h1>{title}</h1>
      <p>{text}</p>
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
    const engine = new Engine()
    for (const bytes of fonts) engine.addFont(bytes)
    const editor = new Editor(engine)
    let failed = ''
    try {
      if (saved) editor.load(saved.bytes, { name: saved.name, handle: saved.handle }, saved.dirty)
    } catch (e) {
      failed = `Could not restore ${saved!.name}: ${(e as Error).message}. Satz started a new document.`
    }
    root.render(
      <StrictMode>
        <App ck={ck} editor={editor} notice={failed} />
      </StrictMode>,
    )
  } catch (e) {
    console.error(e)
    notice('Satz could not start', 'Loading the engine failed. Check your connection and reload the page.')
  }
}
