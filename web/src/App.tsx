import type { Engine } from './engine/engine'
import type { Snapshot } from './model'

export function App({ engine }: { engine: Engine }) {
  const snapshot: Snapshot = engine.snapshot()
  return <pre>{JSON.stringify(snapshot, null, 2)}</pre>
}
