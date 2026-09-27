import { useEffect, useState } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import { Canvas, isTyping } from './Canvas'
import { useEditor, type Editor } from './editor'
import { autosave, download, open, pdf, placeImages, save } from './file'
import { Icon } from './icons'
import { handleKey } from './keys'
import { Layers } from './Layers'
import { Pages } from './Pages'
import { Preflight } from './Preflight'
import { Properties } from './Properties'
import { Start } from './Start'
import { Swatches } from './Swatches'
import { Toolbar } from './Toolbar'

export function App({ ck, editor, first }: { ck: CanvasKit; editor: Editor; first: boolean }) {
  const status = useEditor(editor, (e) => e.status)
  const say = editor.say
  const dirty = useEditor(editor, (e) => e.dirty)
  const name = useEditor(editor, (e) => e.file.name)

  const [exporting, setExporting] = useState(false)
  const [starting, setStarting] = useState(first)
  const [started, setStarted] = useState(!first)
  const [hidden, setHidden] = useState({ left: false, right: false, ui: false })
  const hide = (panel: keyof typeof hidden) => setHidden((h) => ({ ...h, [panel]: !h[panel] }))

  const exportPdf = () => {
    if (exporting) return
    const issues = editor.snapshot.preflight.length
    setExporting(true)
    say('Exporting PDF…')
    pdf(editor)
      .then(
        (bytes) => {
          download(bytes, 'satz.pdf', 'application/pdf')
          say(issues ? `Exported with ${issues} preflight ${issues === 1 ? 'issue' : 'issues'}. See Preflight.` : '')
        },
        (e: Error) => say(`Could not export the PDF: ${e.message}. Reload the page and try again.`),
      )
      .finally(() => setExporting(false))
  }
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
  }, [say, status])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey
      if (e.defaultPrevented || document.querySelector('dialog:modal')) return
      if (e.altKey && !mod && e.code === 'Digit1') hide('left')
      else if (e.altKey && !mod && e.code === 'Digit2') hide('right')
      else if (mod && e.code === 'Backslash') hide('ui')
      else if (mod && e.shiftKey && e.code === 'KeyE') exportPdf()
      else if (mod && !e.altKey && e.code === 'KeyS') saveFile(e.shiftKey)
      else if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyO') open(editor, say).catch(() => {})
      else if (mod && !e.shiftKey && e.code === 'KeyN') setStarting(true)
      else if (mod && e.shiftKey && !e.altKey && e.code === 'KeyK') placeImages(editor, say)
      else if (isTyping(e) || !handleKey(editor, e)) return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  const off = (panel: 'left' | 'right') => hidden.ui || hidden[panel]
  const toggle = (panel: 'left' | 'right', label: string, key: string) => (
    <button
      type="button"
      className="tool toggle"
      aria-label={label}
      title={`${label} (${key})`}
      aria-pressed={!hidden[panel]}
      onClick={() => hide(panel)}
    >
      <Icon name={panel === 'left' ? 'panelLeft' : 'panelRight'} />
    </button>
  )

  return (
    <main className={`app${off('left') ? ' no-left' : ''}${off('right') ? ' no-right' : ''}${hidden.ui ? ' no-ui' : ''}`}>
      <div className="left" inert={off('left')}>
        <div className="brand">
          <span className="mark" aria-hidden="true">
            <i />
            <i />
            <i />
          </span>
          <h1>{name}</h1>
        </div>
        <Pages editor={editor} />
        <Layers editor={editor} />
        <Swatches editor={editor} />
        <Preflight editor={editor} />
      </div>
      <header className="bar" inert={hidden.ui}>
        <Toolbar editor={editor} onPlaceImage={() => placeImages(editor, say)} />
        <span className="sep" />
        {toggle('left', 'Left panel', 'Alt+1')}
        {toggle('right', 'Right panel', 'Alt+2')}
        <span className="grow" />
        <button type="button" className="primary" title="Export PDF (Ctrl+Shift+E)" onClick={exportPdf} disabled={exporting}>
          Export
        </button>
      </header>
      <Canvas ck={ck} editor={editor} />
      <div className="right" inert={off('right')}>
        <Properties editor={editor} say={say} />
      </div>
      {starting && (
        <Start
          editor={editor}
          first={!started}
          say={say}
          onClose={() => {
            setStarting(false)
            setStarted(true)
          }}
        />
      )}
      <p className="status" role="status">
        {status}
      </p>
    </main>
  )
}
