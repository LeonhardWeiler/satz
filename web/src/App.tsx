import { useEffect, useState } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import { Canvas, isTyping } from './Canvas'
import { NameInput } from './controls'
import { ACTIONS, keyLabel, MENU, press } from './commands'
import { ContextMenu } from './ContextMenu'
import { useEditor, type Editor } from './editor'
import { Help } from './Help'
import { autosave, download, drop, open, pdf, placeImages, save } from './file'
import { Icon } from './icons'
import { handleKey } from './keys'
import { Layers } from './Layers'
import { Overview } from './Overview'
import { Palette } from './Palette'
import { isError, Preflight } from './Preflight'
import { Properties } from './Properties'
import { Start } from './Start'
import { Swatches } from './Swatches'
import { Toolbar } from './Toolbar'
import { Tooltip } from './Tooltip'

export function App({ ck, editor, first }: { ck: CanvasKit; editor: Editor; first: boolean }) {
  const status = useEditor(editor, (e) => e.status)
  const say = editor.say
  const dirty = useEditor(editor, (e) => e.dirty)
  const name = useEditor(editor, (e) => e.file.name)
  const overview = useEditor(editor, (e) => e.overview !== null)
  const preflight = useEditor(editor, (e) => e.preflight)
  const issues = useEditor(editor, (e) => e.snapshot.preflight)
  const errors = issues.filter(isError).length

  const [exporting, setExporting] = useState(false)
  const [starting, setStarting] = useState(first)
  const [started, setStarted] = useState(!first)
  const [dialog, setDialog] = useState<'palette' | 'help' | null>(null)
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null)
  const [renaming, setRenaming] = useState(false)
  const selected = useEditor(editor, (e) => e.selection.length > 0)
  const [hidden, setHidden] = useState({ left: false, right: false, ui: false })
  const hide = (panel: keyof typeof hidden) => setHidden((h) => ({ ...h, [panel]: !h[panel] }))

  const exportPdf = () => {
    if (exporting) return
    const title = name.replace(/\.satz$/, '')
    const file = `${title}.pdf`
    setExporting(true)
    say('Exporting PDF…')
    pdf(editor, title)
      .then(
        (bytes) => {
          download(bytes, file, 'application/pdf')
          say(`Exported ${file}`)
        },
        (e: Error) => say(`Could not export the PDF: ${e.message}. Reload the page and try again.`),
      )
      .finally(() => setExporting(false))
  }
  /** Opens the preflight at its export button. */
  const openExport = () => {
    if (!editor.preflight) editor.togglePreflight()
    setHidden((h) => ({ ...h, right: false, ui: false }))
    requestAnimationFrame(() => document.querySelector<HTMLElement>('.pf-go')?.focus())
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
      else if (mod && e.shiftKey && e.code === 'KeyE') openExport()
      else if (mod && e.altKey && e.code === 'KeyY') editor.togglePreflight()
      else if (mod && !e.altKey && e.code === 'KeyS') saveFile(e.shiftKey)
      else if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyO') open(editor, say)
      else if (mod && !e.shiftKey && e.code === 'KeyN') setStarting(true)
      else if (mod && e.shiftKey && !e.altKey && e.code === 'KeyK') placeImages(editor, say)
      else if (mod && !e.shiftKey && !e.altKey && e.code === 'KeyK') setDialog('palette')
      else if (isTyping(e) || (e.target as Element).closest?.('.menu')) return
      else if (e.key === '.' && !mod) editor.toggleOverview()
      else if (e.key === '?' && !mod) setDialog('help')
      else if (editor.overview && e.key === 'Escape') editor.set({ overview: null })
      else if (editor.preflight && e.key === 'Escape') editor.togglePreflight()
      else if ((editor.overview && !mod) || !handleKey(editor, e)) return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  useEffect(() => {
    const onMenu = (e: MouseEvent) => {
      if (e.defaultPrevented || isTyping(e)) return
      e.preventDefault()
      setMenu({ x: e.clientX, y: e.clientY })
    }
    window.addEventListener('contextmenu', onMenu)
    return () => window.removeEventListener('contextmenu', onMenu)
  }, [])

  useEffect(() => {
    const clip = (e: ClipboardEvent) => {
      if (e.defaultPrevented || isTyping(e) || !e.clipboardData) return
      const ids = editor.selection
      if (e.type === 'paste') {
        const images = [...e.clipboardData.files].filter((f) => f.type === 'image/png' || f.type === 'image/jpeg')
        const text = e.clipboardData.getData('text/plain').replace(/\r\n?/g, '\n')
        if (images.length) drop(editor, images, say)
        else if (text.trim() && text !== editor.copied) editor.pasteText(text)
        else editor.set({ selection: editor.apply({ type: 'paste', above: ids, page: editor.page.id }) })
      } else {
        if (!ids.length) return
        e.clipboardData.setData('text/plain', editor.copy(ids))
        if (e.type === 'cut') editor.apply({ type: 'delete', ids })
      }
      e.preventDefault()
    }
    const types = ['copy', 'cut', 'paste'] as const
    types.forEach((t) => window.addEventListener(t, clip))
    return () => types.forEach((t) => window.removeEventListener(t, clip))
  }, [editor, say])

  useEffect(() => {
    const files = (e: DragEvent) => e.dataTransfer?.types.includes('Files')
    const over = (e: DragEvent) => files(e) && e.preventDefault()
    const onDrop = (e: DragEvent) => {
      if (!files(e)) return
      e.preventDefault()
      drop(editor, [...e.dataTransfer!.files], say)
    }
    window.addEventListener('dragover', over)
    window.addEventListener('drop', onDrop)
    return () => {
      window.removeEventListener('dragover', over)
      window.removeEventListener('drop', onDrop)
    }
  }, [editor, say])

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
        <Edge side="left" />
        <div className="brand" onBlur={() => setRenaming(false)}>
          <span className="mark" aria-hidden="true" />
          {renaming ? (
            <NameInput
              label="Document name"
              value={name}
              autoFocus
              onCommit={(v) => editor.saved({ ...editor.file, name: v.endsWith('.satz') ? v : `${v}.satz` }, '')}
            />
          ) : (
            <h1 title="Double-click to rename" onDoubleClick={() => setRenaming(true)}>
              {name}
            </h1>
          )}
        </div>
        <Layers editor={editor} />
        <Swatches editor={editor} />
      </div>
      <header className="bar" inert={hidden.ui}>
        <Toolbar editor={editor} onPlaceImage={() => placeImages(editor, say)} />
        <span className="sep" />
        {toggle('left', 'Left panel', 'Alt+1')}
        {toggle('right', 'Right panel', 'Alt+2')}
        <button
          type="button"
          className="tool toggle"
          aria-label="Page overview"
          title="Page overview (.)"
          aria-pressed={overview}
          onClick={() => editor.toggleOverview()}
        >
          <Icon name="pages" />
        </button>
        <span className="grow" />
        <button
          type="button"
          className="tool pf-btn"
          aria-label={`Preflight, ${issues.length} issue${issues.length === 1 ? '' : 's'}`}
          title="Preflight (Ctrl+Alt+Y)"
          aria-pressed={preflight}
          onClick={() => editor.togglePreflight()}
        >
          <Icon name="preflight" />
          <span className="pf-count" data-sev={errors ? 'error' : issues.length ? 'warn' : 'ok'}>
            {issues.length}
          </span>
        </button>
        <button type="button" className="tool" aria-label="Keyboard shortcuts" title="Keyboard shortcuts (?)" onClick={() => setDialog('help')}>
          <Icon name="help" />
        </button>
        <button type="button" className="primary" title="Export PDF (Ctrl+Shift+E)" onClick={openExport}>
          Export
        </button>
      </header>
      <Canvas ck={ck} editor={editor} />
      {overview && <Overview ck={ck} editor={editor} />}
      <div className="right" inert={off('right')}>
        <Edge side="right" />
        {preflight ? <Preflight editor={editor} exporting={exporting} onExport={exportPdf} /> : <Properties editor={editor} say={say} />}
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
      {dialog === 'palette' && <Palette editor={editor} onClose={() => setDialog(null)} />}
      {dialog === 'help' && <Help onClose={() => setDialog(null)} />}
      <Tooltip />
      {menu && (
        <ContextMenu
          anchor={() => new DOMRect(menu.x, menu.y)}
          label="Actions"
          onClose={() => setMenu(null)}
          items={MENU[selected ? 'selected' : 'none'].map((title) => {
            const a = title && ACTIONS.find((a) => a.title === title)!
            return a ? [a.title, () => (a.run ? a.run(editor) : press(a.keys)), a.can?.(editor) ?? true, undefined, keyLabel(a.keys)] : null
          })}
        />
      )}
      <p className="status" role="status">
        {status}
      </p>
    </main>
  )
}

const WIDTHS = 'satz.panels'
const widths: Record<string, number> = JSON.parse(localStorage.getItem(WIDTHS) ?? '{}')
let grab = 0
for (const [side, w] of Object.entries(widths)) document.documentElement.style.setProperty(`--${side}-width`, `${w}px`)

/** The inner edge of a side panel, dragged to set its width. */
function Edge({ side }: { side: 'left' | 'right' }) {
  const at = (x: number) => (side === 'left' ? x : innerWidth - x)
  return (
    <div
      className="edge"
      aria-hidden="true"
      onPointerDown={(e) => {
        grab = at(e.clientX) - e.currentTarget.parentElement!.offsetWidth
        e.currentTarget.setPointerCapture(e.pointerId)
        e.preventDefault()
      }}
      onPointerMove={(e) => {
        if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
        const w = Math.round(Math.min(560, Math.max(200, at(e.clientX) - grab)))
        document.documentElement.style.setProperty(`--${side}-width`, `${w}px`)
        widths[side] = w
      }}
      onPointerUp={() => localStorage.setItem(WIDTHS, JSON.stringify(widths))}
      onDoubleClick={() => {
        document.documentElement.style.removeProperty(`--${side}-width`)
        delete widths[side]
        localStorage.setItem(WIDTHS, JSON.stringify(widths))
      }}
    />
  )
}
