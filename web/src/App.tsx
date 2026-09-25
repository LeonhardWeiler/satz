import { useEffect, useState } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import { Canvas, isTyping } from './Canvas'
import { Editor } from './editor'
import { handleKey } from './keys'
import { Layers } from './Layers'
import { Properties } from './Properties'
import { Swatches } from './Swatches'
import { Toolbar } from './Toolbar'

export function App({ ck, engine }: { ck: CanvasKit; engine: Engine }) {
  const [editor] = useState(() => new Editor(engine))

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
      if (e.defaultPrevented || document.querySelector('dialog:modal')) return
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.code === 'KeyE') exportPdf()
      else if (isTyping(e) || !handleKey(editor, e)) return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  return (
    <main className="app">
      <h1 className="sr-only">Satz</h1>
      <div className="left">
        <Layers editor={editor} />
        <Swatches editor={editor} />
      </div>
      <Canvas ck={ck} editor={editor} />
      <Properties editor={editor} onExport={exportPdf} />
      <Toolbar editor={editor} />
    </main>
  )
}
