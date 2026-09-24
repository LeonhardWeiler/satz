import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import CanvasKitInit from 'canvaskit-wasm'
import canvaskitWasm from 'canvaskit-wasm/bin/canvaskit.wasm?url'
import init, { Engine } from './engine/engine'
import { App } from './App'
import './index.css'

const root = createRoot(document.getElementById('root')!)

if (!document.createElement('canvas').getContext('webgl2')) {
  root.render(
    <main className="notice" role="alert">
      <h1>WebGL 2 is not available</h1>
      <p>Satz draws the page with WebGL 2. Turn on hardware acceleration in your browser settings, or open Satz in a current Chrome or Firefox.</p>
    </main>,
  )
} else {
  const [ck] = await Promise.all([CanvasKitInit({ locateFile: () => canvaskitWasm }), init()])
  root.render(
    <StrictMode>
      <App ck={ck} engine={new Engine()} />
    </StrictMode>,
  )
}
