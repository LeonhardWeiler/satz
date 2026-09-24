import type { CanvasKit } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { Canvas } from './Canvas'

export function App({ ck, engine }: { ck: CanvasKit; engine: Engine }) {
  return (
    <main className="app">
      <Canvas ck={ck} engine={engine} />
    </main>
  )
}
