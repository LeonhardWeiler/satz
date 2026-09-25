import { useEffect, useState } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import { Canvas, isTyping } from './Canvas'
import { useEditor, type Editor } from './editor'
import { autosave, download, open, save, start } from './file'
import { handleKey } from './keys'
import { Layers } from './Layers'
import { Pages } from './Pages'
import { Properties } from './Properties'
import { Swatches } from './Swatches'
import { Toolbar } from './Toolbar'

export function App({ ck, editor, notice = '' }: { ck: CanvasKit; editor: Editor; notice?: string }) {
  const [status, say] = useState(notice)
  const dirty = useEditor(editor, (e) => e.dirty)
  const name = useEditor(editor, (e) => e.file.name)

  const exportPdf = () => download(editor.engine.pdf(), 'satz.pdf', 'application/pdf')
  const saveFile = (as: boolean) => {
    say('Saving…')
    save(editor, as).then(
      () => say(`Saved ${editor.file.name}`),
      (e: Error) => say(e.name === 'AbortError' ? '' : `Could not save ${name}: ${e.message}. Try Save as (Ctrl+Shift+S).`),
    )
  }

  useEffect(() => autosave(editor), [editor])
  useEffect(() => {
    document.title = `${dirty ? '* ' : ''}${name} — Satz`
  }, [dirty, name])
  useEffect(() => {
    const t = setTimeout(() => say(''), 5000)
    return () => clearTimeout(t)
  }, [status])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey
      if (e.defaultPrevented || document.querySelector('dialog:modal')) return
      if (mod && e.shiftKey && e.code === 'KeyE') exportPdf()
      else if (mod && !e.altKey && e.code === 'KeyS') saveFile(e.shiftKey)
      else if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyO') open(editor, say).catch(() => {})
      else if (mod && e.altKey && !e.shiftKey && e.code === 'KeyN') start(editor)
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
        <Pages editor={editor} />
        <Layers editor={editor} />
        <Swatches editor={editor} />
      </div>
      <Canvas ck={ck} editor={editor} />
      <Properties editor={editor} onExport={exportPdf} />
      <Toolbar editor={editor} />
      <p className="status" role="status">
        {status}
      </p>
    </main>
  )
}
