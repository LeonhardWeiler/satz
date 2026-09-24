import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import CanvasKitInit from 'canvaskit-wasm'
import canvaskitWasm from 'canvaskit-wasm/bin/canvaskit.wasm?url'
import init, { Engine } from './engine/engine'
import { App } from './App'
import './index.css'

const [ck] = await Promise.all([CanvasKitInit({ locateFile: () => canvaskitWasm }), init()])

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App ck={ck} engine={new Engine()} />
  </StrictMode>,
)
