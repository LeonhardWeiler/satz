import { useEffect, useRef, useState } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import { Canvas, isTyping } from './Canvas'
import { Edge, Keys, NameInput } from './controls'
import { ACTIONS, addPage, comboOf, keysOf, MENU, ONLY_WHEN_ABLE, press } from './commands'
import { ContextMenu } from './ContextMenu'
import { CLIP, useEditor, type Editor } from './editor'
import { Settings } from './SettingsDialog'
import { Find } from './Find'
import { History } from './History'
import { settings } from './settings'
import { autosave, download, drop, exportFile, flush, open, placeImages, save, show, type Handle } from './file'
import { Icon } from './icons'
import { handleKey } from './keys'
import { Layers } from './Layers'
import { Overview } from './Overview'
import { Palette } from './Palette'
import { type Format, isError, Preflight } from './Preflight'
import { Properties } from './Properties'
import { Start } from './Start'
import { Home } from './Home'
import { Assets } from './Assets'
import { Swatches } from './Swatches'
import { Switcher } from './Switcher'
import { Toolbar } from './Toolbar'
import { Tooltip } from './Tooltip'

const SHOWS: Record<string, [label: string, panel: 'rulers' | 'left' | 'right']> = {
  'Show or hide rulers': ['Rulers', 'rulers'],
  'Show or hide left panel': ['Left panel', 'left'],
  'Show or hide right panel': ['Right panel', 'right'],
}

export function App({ ck, editor }: { ck: CanvasKit; editor: Editor }) {
  const status = useEditor(editor, (e) => e.status)
  const statusError = useEditor(editor, (e) => e.statusError)
  const statusAction = useEditor(editor, (e) => e.statusAction)
  const say = editor.say
  const dirty = useEditor(editor, (e) => e.dirty)
  const name = useEditor(editor, (e) => e.file.name)
  const title = name.replace(/\.satz$/, '')
  const overview = useEditor(editor, (e) => e.overview !== null)
  const preflight = useEditor(editor, (e) => e.preflight)
  const issues = useEditor(editor, (e) => e.snapshot.preflight)
  const errors = issues.filter(isError).length
  const home = useEditor(editor, (e) => e.home)

  const [exporting, setExporting] = useState(false)
  const lastFormat = useRef<Format>('pdf')
  const [starting, setStarting] = useState(false)
  const [dialog, setDialog] = useState<'palette' | 'help' | 'settings' | 'history' | null>(null)
  const [finding, setFinding] = useState(false)
  const [menu, setMenu] = useState<{ x: number; y: number; sub?: 'transform' | 'table' } | null>(null)
  const [renaming, setRenaming] = useState(false)
  const [tab, setTab] = useState<'Layers' | 'Assets'>('Layers')
  const selected = useEditor(editor, (e) => e.selection.length > 0)
  const picked = useEditor(editor, (e) => e.selection.join())
  const [hidden, setHidden] = useState({ left: false, right: false, ui: false, rulers: false })
  const hide = (panel: keyof typeof hidden) => setHidden((h) => ({ ...h, [panel]: !h[panel] }))

  const exportAs = (format: Format, pages: number[]) => {
    if (exporting) return
    lastFormat.current = format
    const kind = format === 'zip' ? 'package' : format.toUpperCase()
    const image = format === 'png' || format === 'jpeg'
    const zipped = image && pages.length > 1
    const ext = zipped ? 'zip' : format === 'jpeg' ? 'jpg' : format
    const type = image && !zipped ? `image/${format}` : `application/${ext}`
    setExporting(true)
    say(`Exporting ${kind}…`)
    exportFile(editor, image ? { type: `image/${format}`, ppi: editor.snapshot.rasterPpi, title, pages } : { type: `application/${format}`, title, pages })
      .then(
        (bytes) => {
          const file = image && !zipped && editor.snapshot.pages.length > 1 ? `${title}-${pages[0] + 1}.${ext}` : `${title}.${ext}`
          download(bytes, file, type)
          say(`Exported ${zipped ? `${pages.length} pages as ${file}` : file}`)
        },
        (e: Error) => editor.fail(`Could not export the ${kind}: ${e.message}. Reload the page and try again.`),
      )
      .finally(() => setExporting(false))
  }
  /** Exports every page in the last format, or opens the preflight at its export button when it has errors. */
  const quickExport = () => {
    if (!editor.snapshot.preflight.some(isError)) return exportAs(lastFormat.current, editor.snapshot.pages.map((_, i) => i))
    if (!editor.preflight) editor.togglePreflight()
    setHidden((h) => ({ ...h, right: false, ui: false }))
    requestAnimationFrame(() => document.querySelector<HTMLElement>('.pf-go')?.focus())
  }
  const saveFile = (as: boolean) => {
    say('Saving…')
    save(editor, as).then(
      () => say(`Saved ${editor.file.name}`),
      (e: Error) => e.name === 'AbortError' ? say('') : editor.fail(<>Could not save {name}: {e.message}. Try Save as <Keys keys="Ctrl Shift S" /></>),
    )
  }

  useEffect(() => autosave(editor), [editor])
  useEffect(() => {
    document.title = `${dirty ? '* ' : ''}${name} - Satz`
  }, [dirty, name])
  useEffect(() => {
    const t = setTimeout(() => say(''), 5000)
    return () => clearTimeout(t)
  }, [say, status])

  useEffect(() => {
    const remap = (e: KeyboardEvent) => {
      const keys = comboOf(e)
      if (!e.isTrusted || !keys || document.querySelector('dialog:modal') || (isTyping(e) && !/Ctrl|Alt/.test(keys))) return
      const to = ACTIONS.find((a) => settings.keys[a.title] === keys)
      if (!to && !ACTIONS.some((a) => a.keys === keys && a.title in settings.keys)) return
      e.preventDefault()
      e.stopImmediatePropagation()
      if (to) press(to.keys)
    }
    window.addEventListener('keydown', remap, true)
    return () => window.removeEventListener('keydown', remap, true)
  }, [])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey
      if (e.defaultPrevented || document.querySelector('dialog:modal')) return
      if (e.altKey && !mod && e.code === 'Digit1') hide('left')
      else if (e.altKey && !mod && e.code === 'Digit2') hide('right')
      else if (mod && e.shiftKey && e.code === 'KeyE') quickExport()
      else if (mod && e.altKey && e.code === 'KeyY') editor.togglePreflight()
      else if (mod && !e.altKey && e.code === 'KeyS') saveFile(e.shiftKey)
      else if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyO') open(editor)
      else if (mod && e.altKey && !e.shiftKey && e.code === 'KeyN') setStarting(true)
      else if (mod && e.shiftKey && !e.altKey && e.code === 'KeyK') placeImages(editor)
      else if (mod && e.altKey && !e.shiftKey && e.code === 'KeyP') addPage(editor)
      else if (mod && !e.shiftKey && !e.altKey && e.code === 'KeyK') setDialog('palette')
      else if (mod && !e.shiftKey && e.code === 'Comma') setDialog('settings')
      else if (mod && e.altKey && !e.shiftKey && e.code === 'KeyZ') setDialog('history')
      else if (mod && !e.shiftKey && !e.altKey && e.code === 'KeyF') setFinding(true)
      else if (isTyping(e) || (e.target as Element).closest?.('.menu')) return
      else if (e.shiftKey && !mod && !e.altKey && e.code === 'KeyR') hide('rulers')
      else if (e.shiftKey && !mod && !e.altKey && e.code === 'KeyW') {
        if (!hidden.ui) editor.say(<><Keys keys="Shift W" /> shows the interface again</>)
        hide('ui')
      } else if (e.key === '.' && !mod) editor.toggleOverview()
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
      if (!(e.target as Element).closest('.stage, .layers')) editor.set({ selection: [] })
      setMenu({ x: e.clientX, y: e.clientY })
    }
    window.addEventListener('contextmenu', onMenu)
    return () => window.removeEventListener('contextmenu', onMenu)
  }, [editor])

  useEffect(() => {
    const clip = (e: ClipboardEvent) => {
      if (e.defaultPrevented || isTyping(e) || !e.clipboardData) return
      const ids = editor.selection
      if (e.type === 'paste') {
        const images = [...e.clipboardData.files].filter((f) => f.type === 'image/png' || f.type === 'image/jpeg')
        const text = e.clipboardData.getData('text/plain').replace(/\r\n?/g, '\n')
        const clip = e.clipboardData.getData(CLIP)
        if (clip) editor.paste(clip)
        else if (images.length) drop(editor, images)
        else if (text.trim() && text !== editor.copied) editor.pasteText(text)
        else editor.paste()
      } else {
        if (!ids.length) return
        e.clipboardData.setData('text/plain', editor.copy(ids))
        e.clipboardData.setData(CLIP, editor.clip!)
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
      const items = [...e.dataTransfer!.items].filter((i) => i.kind === 'file') as (DataTransferItem & { getAsFileSystemHandle?(): Promise<Handle | null> })[]
      drop(editor, [...e.dataTransfer!.files], items.map((i) => i.getAsFileSystemHandle?.() ?? Promise.resolve(null)))
    }
    window.addEventListener('dragover', over)
    window.addEventListener('drop', onDrop)
    return () => {
      window.removeEventListener('dragover', over)
      window.removeEventListener('drop', onDrop)
    }
  }, [editor, say])

  const off = (panel: 'left' | 'right') => hidden.ui || hidden[panel]
  const bar = (
    <p
      className={statusError ? 'status error' : 'status'}
      role="status"
    >
      {statusError && <Icon name="error" />}
      {status}
      {statusAction && (
        <button
          type="button"
          className="button"
          onClick={() => {
            say('')
            statusAction.run()
          }}
        >
          {statusAction.label}
        </button>
      )}
    </p>
  )

  return (
    <main className={`app${off('left') ? ' no-left' : ''}${off('right') ? ' no-right' : ''}${hidden.ui ? ' no-ui' : ''}${hidden.rulers ? ' no-rulers' : ''}`}>
      <div className="left" inert={off('left')}>
        <Edge side="left" />
        <div className="brand" title="Double-click to rename" onDoubleClick={() => setRenaming(true)} onBlur={() => setRenaming(false)}>
          <button type="button" className="mark" aria-label="Projects" title="Projects" onClick={() => flush().then(() => show(editor, null))} />
          {renaming ? (
            <>
              <NameInput label="Document name" value={title} autoFocus onCommit={(v) => editor.saved({ ...editor.file, name: `${v}.satz` }, '')} />
              <span className="ext">.satz</span>
            </>
          ) : (
            <h1>
              {title}
              <span className="ext">.satz</span>
            </h1>
          )}
        </div>
        <div className="tabs" role="tablist" aria-label="Left panel">
          {(['Layers', 'Assets'] as const).map((t) => (
            <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
              {t}
            </button>
          ))}
        </div>
        {tab === 'Layers' ? <Layers editor={editor} /> : <Assets editor={editor} />}
        <div className="left-swatches" onPointerDown={editor.gesture}>
          <Edge side="swatches" />
          <Swatches editor={editor} />
        </div>
      </div>
      <header className="bar" inert={hidden.ui}>
        <div className="bar-side">
          {off('left') && (
            <>
              <span className="bar-name">
                {title}
                <span className="ext">.satz</span>
              </span>
              <span className="sep" />
            </>
          )}
          {!overview && <Toolbar editor={editor} onPlaceImage={() => placeImages(editor)} />}
        </div>
        {overview ? <span /> : <Switcher editor={editor} />}
        <div className="bar-side end">
          <button type="button" className="tool" aria-label="Command palette" title="Search commands, layers and pages (Ctrl+K)" onClick={() => setDialog('palette')}>
            <Icon name="search" />
          </button>
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
          <span className="sep" />
          <button type="button" className="tool" aria-label="Version history" title="Version history (Ctrl+Alt+Z)" onClick={() => setDialog('history')}>
            <Icon name="clock" />
          </button>
          <button type="button" className="tool" aria-label="Settings" title="Settings (Ctrl+,)" onClick={() => setDialog('settings')}>
            <Icon name="settings" />
          </button>
          <button
            type="button"
            className="tool pf-count"
            data-sev={errors ? 'error' : issues.length ? 'warn' : undefined}
            aria-label={issues.length ? `Preflight, ${issues.length} issue${issues.length === 1 ? '' : 's'}` : 'Preflight and export options'}
            title="Preflight and export options (Ctrl+Alt+Y)"
            aria-pressed={preflight}
            onClick={() => editor.togglePreflight()}
          >
            <Icon name={issues.length ? 'warn' : 'ok'} />
            {issues.length > 0 && (errors || issues.length)}
          </button>
          {!preflight && (
            <button type="button" className="primary" title={errors ? 'Export: shows the errors first (Ctrl+Shift+E)' : 'Export every page (Ctrl+Shift+E)'} onClick={quickExport}>
              Export
            </button>
          )}
        </div>
      </header>
      <Canvas ck={ck} editor={editor} />
      {overview && <Overview ck={ck} editor={editor} />}
      <div className="right" inert={off('right')}>
        <Edge side="right" />
        {preflight ? <Preflight editor={editor} exporting={exporting} onExport={exportAs} /> : <Properties key={picked} editor={editor} />}
      </div>
      {starting && (
        <Start editor={editor} onClose={() => setStarting(false)} />
      )}
      {finding && <Find editor={editor} onClose={() => setFinding(false)} />}
      {dialog === 'palette' && <Palette editor={editor} onClose={() => setDialog(null)} />}
      {dialog === 'history' && <History editor={editor} onClose={() => setDialog(null)} />}
      {(dialog === 'settings' || dialog === 'help') && <Settings page={dialog === 'help' ? 'shortcuts' : 'general'} onClose={() => setDialog(null)} />}
      <Tooltip />
      {menu && (
        <ContextMenu
          anchor={() => new DOMRect(menu.x, menu.y)}
          label="Actions"
          onClose={() => setMenu(null)}
          items={MENU[menu.sub ?? (selected ? 'selected' : 'none')]
            .filter((t) => !t || !ONLY_WHEN_ABLE.has(t) || (t === 'Table' ? MENU.table : [t]).some((t) => ACTIONS.find((a) => a.title === t)!.can!(editor)))
            .map((title) => {
            if (title === 'Transform' || title === 'Table') return [title, () => setTimeout(() => setMenu({ ...menu, sub: title.toLowerCase() as 'transform' | 'table' })), true, undefined, '›']
            const a = title && ACTIONS.find((a) => a.title === title)!
            const shows = title && SHOWS[title]
            if (a && shows) return [shows[0], () => hide(shows[1]), true, !hidden[shows[1]], keysOf(a)]
            return a ? [a.title, () => (a.run ? a.run(editor) : press(a.keys)), a.can?.(editor) ?? true, undefined, keysOf(a)] : null
          })}
        />
      )}
      {home ? (
        <Home editor={editor} onNew={() => setStarting(true)}>
          {bar}
        </Home>
      ) : (
        bar
      )}
    </main>
  )
}
