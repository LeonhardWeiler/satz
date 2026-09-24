import { useEffect } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { Canvas } from './Canvas'

export function App({ ck, engine }: { ck: CanvasKit; engine: Engine }) {
  const exportPdf = () => {
    const url = URL.createObjectURL(new Blob([engine.pdf() as Uint8Array<ArrayBuffer>], { type: 'application/pdf' }))
    const a = document.createElement('a')
    a.href = url
    a.download = 'satz.pdf'
    a.click()
    URL.revokeObjectURL(url)
  }

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.code === 'KeyE') {
        e.preventDefault()
        exportPdf()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  return (
    <main className="app">
      <Canvas ck={ck} engine={engine} />
      <button type="button" className="export" onClick={exportPdf} title="Export PDF (Ctrl+Shift+E)">
        Export PDF
      </button>
    </main>
  )
}
