import { useEffect, useRef, useState } from 'react'
import { Engine } from './engine/engine'
import { MM, type Editor } from './editor'
import { begin, discard, open } from './file'
import { Field, nextName, Segmented } from './controls'
import { length, setSettings, useSettings, type DocPreset } from './settings'
import { Icon } from './icons'
import type { Page } from './types'

export const ORIENTATIONS = [
  ['portrait', 'Portrait', 'portrait'],
  ['landscape', 'Landscape', 'landscape'],
] as const
const MODES = [
  ['cmyk', 'CMYK'],
  ['rgb', 'RGB'],
] as const
export const FORMATS: [string, number, number][] = [
  ['A2', 420, 594],
  ['A3', 297, 420],
  ['A4', 210, 297],
  ['A5', 148, 210],
  ['A6', 105, 148],
  ['B4', 250, 353],
  ['B5', 176, 250],
  ['DL', 99, 210],
  ['Square', 210, 210],
  ['Business card', 55, 85],
  ['Letter', 215.9, 279.4],
  ['Legal', 215.9, 355.6],
  ['Tabloid', 279.4, 431.8],
]
const near = (a: number, b: number) => Math.abs(a - b) < 0.5
/** The name of the format of a `w` by `h` pt page in either orientation, or Custom. */
export const formatOf = (w: number, h: number) =>
  FORMATS.find(([, a, b]) => near(Math.min(w, h), a * MM) && near(Math.max(w, h), b * MM))?.[0] ?? 'Custom'

const A5: DocPreset = { name: '', w: 148 * MM, h: 210 * MM, pages: 8, facing: true, mode: 'cmyk', margin: 15 * MM, columns: 1, gutter: 5 * MM, bleed: 3 * MM }
const plural = (n: number) => `${n} ${n === 1 ? 'page' : 'pages'}`
const mm = (pt: number) => length(pt).split(' ')[0]

function Tile({ name, w, h, checked, detail, onPick }: { name: string; w: number; h: number; checked: boolean; detail: string; onPick: () => void }) {
  const k = 64 / (594 * MM)
  return (
    <button type="button" role="radio" className="format" aria-checked={checked} tabIndex={checked ? 0 : -1} onClick={onPick}>
      <span className="format-box">
        <span style={{ width: Math.min(w * k, 64), height: Math.min(h * k, 64) }} />
      </span>
      <strong>{name}</strong>
      <small>{detail}</small>
    </button>
  )
}

export function Start({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const saved = useSettings().docPresets
  const [doc, setDoc] = useState(A5)
  const [preset, setPreset] = useState<string | null>(null)
  const [name, setName] = useState(() => nextName('Preset', saved.map((p) => p.name)))
  useEffect(() => {
    ref.current!.showModal()
    ref.current!.querySelector<HTMLElement>('[role=radio][aria-checked=true]')?.focus()
  }, [])

  const { w, h, pages } = doc
  const land = w > h
  const format = formatOf(w, h)
  const set = (patch: Partial<DocPreset>) => {
    setDoc({ ...doc, ...patch })
    setPreset(null)
  }
  const pick = (p: DocPreset) => {
    setDoc(p)
    setPreset(p.name)
    setName(p.name)
  }
  const size = (a: number, b: number) => (land ? [Math.max(a, b), Math.min(a, b)] : [Math.min(a, b), Math.max(a, b)])

  const create = async () => {
    if (!(await discard(editor))) return
    const e = Engine.blank(w, h, pages, pages > 1 && doc.facing)
    if (doc.mode !== 'cmyk') e.apply({ type: 'setDocument', colorMode: doc.mode })
    for (const p of JSON.parse(e.snapshot()).pages as Page[]) {
      e.apply({ type: 'setPage', id: p.id, bleed: doc.bleed })
      if (doc.margin > 0 || doc.columns > 1) e.apply({ type: 'setGrids', id: p.id, grids: [{ kind: 'columns', count: doc.columns, gutter: doc.gutter, margin: doc.margin, size: 5 * MM }] })
    }
    await begin(editor, e.save())
    e.free()
    editor.say(`Created ${format}, ${plural(pages)}`)
    onClose()
  }
  const save = () => {
    const n = name.trim() || nextName('Preset', saved.map((p) => p.name))
    setSettings({ docPresets: [...saved.filter((p) => p.name !== n), { ...doc, name: n }] })
    setName(n)
    setPreset(n)
  }

  const openFile = () => {
    onClose()
    open(editor)
  }

  return (
    <dialog
      ref={ref}
      className="start"
      aria-label="New document"
      onCancel={(e) => {
        e.preventDefault()
        onClose()
      }}
      onKeyDown={(e) => {
        const target = e.target as HTMLElement
        const tiles = [...ref.current!.querySelectorAll<HTMLElement>('.format')]
        const at = tiles.indexOf(target)
        if (at >= 0 && ['ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          const next = tiles[Math.max(0, Math.min(tiles.length - 1, at + (e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 1)))]
          next.click()
          next.focus()
        } else if (e.key === 'Enter' && (at >= 0 || target === ref.current)) {
          e.preventDefault()
          create()
        } else if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.code === 'KeyO') {
          e.preventDefault()
          openFile()
        }
      }}
    >
      <header className="start-head">
        <h2>New document</h2>
        <button type="button" className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
          <Icon name="close" />
        </button>
      </header>
      <div className="start-presets">
        {saved.length > 0 && (
          <>
            <h3>Saved</h3>
            <div className="formats" role="radiogroup" aria-label="Saved presets">
              {saved.map((p) => (
                <div key={p.name} className="saved">
                  <Tile name={p.name} w={p.w} h={p.h} checked={preset === p.name} detail={`${formatOf(p.w, p.h)}, ${plural(p.pages)}`} onPick={() => pick(p)} />
                  <button
                    type="button"
                    className="icon-button"
                    aria-label={`Delete preset ${p.name}`}
                    title="Delete preset"
                    onClick={() => setSettings({ docPresets: saved.filter((q) => q !== p) })}
                  >
                    <Icon name="close" />
                  </button>
                </div>
              ))}
            </div>
          </>
        )}
        <h3>Formats</h3>
        <div className="formats" role="radiogroup" aria-label="Format">
          {FORMATS.map(([n, a, b]) => {
            const [fw, fh] = size(a * MM, b * MM)
            return <Tile key={n} name={n} w={fw} h={fh} checked={!preset && format === n} detail={`${mm(fw)} × ${mm(fh)}`} onPick={() => set({ w: fw, h: fh })} />
          })}
        </div>
      </div>
      <section className="start-details" aria-label="Preset details">
        <div className="start-name">
          <label className="field">
            <span className="field-label">Name</span>
            <input aria-label="Preset name" autoComplete="off" value={name} onFocus={(e) => e.currentTarget.select()} onChange={(e) => setName(e.currentTarget.value)} />
          </label>
          <button type="button" className="icon-button" aria-label="Save preset" title="Save preset" onClick={save}>
            <Icon name="plus" />
          </button>
        </div>
        <h3>Size</h3>
        <div className="grid">
          <Field label="W" title="Width" value={w} unit="length" min={1} max={14400} onCommit={(v) => set({ w: v })} />
          <Field label="H" title="Height" value={h} unit="length" min={1} max={14400} onCommit={(v) => set({ h: v })} />
          <Segmented label="Orientation" value={land ? 'landscape' : 'portrait'} options={ORIENTATIONS} onChange={(o) => (o === 'landscape') !== land && set({ w: h, h: w })} />
          <Field label="Pages" title="Pages" value={pages} unit="" int min={1} max={999} onCommit={(v) => set({ pages: Math.round(v) })} />
        </div>
        <label className="check">
          <input type="checkbox" checked={pages > 1 && doc.facing} disabled={pages < 2} onChange={(e) => set({ facing: e.currentTarget.checked })} />
          Facing pages
        </label>
        <h3>Color</h3>
        <Segmented label="Color mode" value={doc.mode} options={MODES} text onChange={(mode) => set({ mode })} />
        <h3>Margins, columns and bleed</h3>
        <div className="grid">
          <Field label="Margin" title="Margin" value={doc.margin} unit="length" onCommit={(margin) => set({ margin })} />
          <Field label="Columns" title="Columns" value={doc.columns} unit="" int min={1} max={24} onCommit={(v) => set({ columns: Math.round(v) })} />
          <Field label="Gutter" title="Gutter" value={doc.gutter} unit="length" onCommit={(gutter) => set({ gutter })} />
          <Field label="Bleed" title="Bleed" value={doc.bleed} unit="length" max={100 * MM} onCommit={(bleed) => set({ bleed })} />
        </div>
      </section>
      <footer className="start-footer">
        <button type="button" className="button" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="primary" title={`${format}, ${plural(pages)}`} onClick={create}>
          Create
        </button>
      </footer>
    </dialog>
  )
}
