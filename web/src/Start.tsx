import { useEffect, useRef, useState } from 'react'
import { Engine } from './engine/engine'
import { MM, type Editor } from './editor'
import { discard, open } from './file'
import { Field, Segmented } from './controls'
import { Icon } from './icons'
import booklet from '../../examples/booklet.satz?url'
import poster from '../../examples/poster.satz?url'

export const ORIENTATIONS = [
  ['portrait', 'Portrait', 'portrait'],
  ['landscape', 'Landscape', 'landscape'],
] as const
export const FORMATS: [string, number, number][] = [
  ['A2', 420, 594],
  ['A3', 297, 420],
  ['A4', 210, 297],
  ['A5', 148, 210],
  ['A6', 105, 148],
  ['DL', 99, 210],
  ['US Letter', 215.9, 279.4],
]
const EXAMPLES = [
  ['Satz booklet', 'A5, 8 pages, master pages', booklet, 'booklet.satz'],
  ['Poster', 'A2, 1 page, CMYK with a spot colour', poster, 'poster.satz'],
  ['Sample', 'A5, 1 page with shapes and text', null, 'Untitled.satz'],
] as const

export function Start({ editor, first, say, onClose }: { editor: Editor; first: boolean; say: (m: string) => void; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [format, setFormat] = useState('A5')
  const [custom, setCustom] = useState([148 * MM, 210 * MM])
  const [land, setLand] = useState(false)
  const [pages, setPages] = useState(8)
  const [facing, setFacing] = useState(true)
  useEffect(() => {
    ref.current!.showModal()
    ref.current!.querySelector<HTMLElement>('[role=radio][aria-checked=true]')?.focus()
  }, [])

  const all: [string, number, number][] = [...FORMATS.map(([n, w, h]): [string, number, number] => [n, w * MM, h * MM]), ['Custom', ...custom] as [string, number, number]]
  const size = ([, w, h]: [string, number, number]) => (land === w < h ? [h, w] : [w, h])
  const [w, h] = size(all.find((f) => f[0] === format)!)
  const plural = (n: number) => `${n} ${n === 1 ? 'page' : 'pages'}`

  const example = async (url: string | null, name: string) => {
    if (!discard(editor)) return
    if (url) editor.load(new Uint8Array(await (await fetch(url)).arrayBuffer()), { name, handle: null })
    else load(new Engine())
    onClose()
  }
  const load = (e: Engine) => {
    editor.load(e.save())
    e.free()
  }
  const create = () => {
    if (!discard(editor)) return
    load(Engine.blank(w, h, pages, pages > 1 && facing))
    say(`Created ${format}, ${plural(pages)}`)
    onClose()
  }
  const pick = (i: number) => {
    const f = all[Math.max(0, Math.min(all.length - 1, i))]
    setFormat(f[0])
    ref.current!.querySelector<HTMLElement>(`[data-format="${f[0]}"]`)?.focus()
  }

  const openFile = () => {
    onClose()
    open(editor, say).catch(() => {})
  }

  return (
    <dialog
      ref={ref}
      className="start"
      aria-label="New document"
      onCancel={(e) => {
        e.preventDefault()
        if (first) example(booklet, 'booklet.satz')
        else onClose()
      }}
      onKeyDown={(e) => {
        const target = e.target as HTMLElement
        const at = all.findIndex((f) => f[0] === target.dataset.format)
        if (at >= 0 && ['ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowDown'].includes(e.key)) {
          e.preventDefault()
          pick(at + (e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 1))
        } else if (e.key === 'Enter' && (at >= 0 || target === ref.current)) {
          e.preventDefault()
          create()
        } else if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.code === 'KeyO') {
          e.preventDefault()
          openFile()
        }
      }}
    >
      <div className="start-main">
        <h2>New document</h2>
        <div className="formats" role="radiogroup" aria-label="Format">
          {all.map((f) => {
            const [a, b] = size(f)
            const k = 64 / (594 * MM)
            return (
              <button
                key={f[0]}
                type="button"
                role="radio"
                className="format"
                data-format={f[0]}
                aria-checked={format === f[0]}
                tabIndex={format === f[0] ? 0 : -1}
                onClick={() => setFormat(f[0])}
              >
                <span className="format-box">
                  <span style={{ width: Math.min(a * k, 64), height: Math.min(b * k, 64) }} />
                </span>
                <strong>{f[0]}</strong>
                <small>
                  {Math.round(a / MM)} × {Math.round(b / MM)} mm
                </small>
              </button>
            )
          })}
        </div>
        <div className="start-options">
          <Segmented label="Orientation" value={land ? 'landscape' : 'portrait'} options={ORIENTATIONS} onChange={(o) => setLand(o === 'landscape')} />
          {format === 'Custom' && (
            <>
              <Field label="W" title="Width" value={w} unit="mm" onCommit={(v) => v > 0 && setCustom(land ? [custom[0], v] : [v, custom[1]])} />
              <Field label="H" title="Height" value={h} unit="mm" onCommit={(v) => v > 0 && setCustom(land ? [v, custom[1]] : [custom[0], v])} />
            </>
          )}
          <Field label="Pages" title="Pages" value={pages} unit="" onCommit={(v) => setPages(Math.max(1, Math.min(999, Math.round(v))))} />
          <label className="check">
            <input type="checkbox" checked={pages > 1 && facing} disabled={pages < 2} onChange={(e) => setFacing(e.currentTarget.checked)} />
            Facing pages
          </label>
        </div>
        <button type="button" className="primary start-create" onClick={create}>
          Create {format}, {plural(pages)}
          <kbd>Enter</kbd>
        </button>
      </div>
      <aside className="start-side">
        <h3>Examples</h3>
        {EXAMPLES.map(([title, text, url, name]) => (
          <button key={title} type="button" className="example" onClick={() => example(url, name)}>
            <span>
              <Icon name="doc" />
              {title}
            </span>
            <small>{text}</small>
          </button>
        ))}
        <button type="button" className="example" onClick={openFile}>
          <span>
            <Icon name="search" />
            Open file…
          </span>
          <small>Ctrl+O</small>
        </button>
        <p>{first ? 'Esc opens the booklet' : 'Esc returns to the document'}</p>
      </aside>
    </dialog>
  )
}
