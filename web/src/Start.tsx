import { useEffect, useRef, useState } from 'react'
import { Engine } from './engine/engine'
import { MM, type Editor } from './editor'
import { begin, discard, open } from './file'
import { Field, Keys, Segmented } from './controls'
import { length } from './settings'
import { Icon } from './icons'

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
  ['Letter', 215.9, 279.4],
]
const near = (a: number, b: number) => Math.abs(a - b) < 0.5
/** The name of the format of a `w` by `h` pt page in either orientation, or Custom. */
export const formatOf = (w: number, h: number) =>
  FORMATS.find(([, a, b]) => near(Math.min(w, h), a * MM) && near(Math.max(w, h), b * MM))?.[0] ?? 'Custom'

export function Start({ editor, onClose }: { editor: Editor; onClose: () => void }) {
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

  const create = async () => {
    if (!(await discard(editor))) return
    const e = Engine.blank(w, h, pages, pages > 1 && facing)
    await begin(editor, e.save())
    e.free()
    editor.say(`Created ${formatOf(w, h)}, ${plural(pages)}`)
    onClose()
  }
  const pick = (i: number) => {
    const f = all[Math.max(0, Math.min(all.length - 1, i))]
    setFormat(f[0])
    ref.current!.querySelector<HTMLElement>(`[data-format="${f[0]}"]`)?.focus()
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
        <header className="start-head">
          <h2>New document</h2>
          <button type="button" className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
            <Icon name="close" />
          </button>
        </header>
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
                  {f[0] === 'Custom' ? (
                    <span className="format-custom">
                      <Icon name="plus" />
                    </span>
                  ) : (
                    <span style={{ width: Math.min(a * k, 64), height: Math.min(b * k, 64) }} />
                  )}
                </span>
                <strong>{f[0]}</strong>
                <small>{f[0] === 'Custom' && format !== 'Custom' ? 'Any size' : `${length(a).split(' ')[0]} × ${length(b).split(' ')[0]}`}</small>
              </button>
            )
          })}
        </div>
        <div className="start-options">
          <Segmented label="Orientation" value={land ? 'landscape' : 'portrait'} options={ORIENTATIONS} onChange={(o) => setLand(o === 'landscape')} />
          {format === 'Custom' && (
            <>
              <Field label="W" title="Width" value={w} unit="length" min={1} max={14400} onCommit={(v) => setCustom(land ? [custom[0], v] : [v, custom[1]])} />
              <Field label="H" title="Height" value={h} unit="length" min={1} max={14400} onCommit={(v) => setCustom(land ? [v, custom[1]] : [custom[0], v])} />
            </>
          )}
          <Field label="Pages" title="Pages" value={pages} unit="" int min={1} max={999} onCommit={(v) => setPages(Math.round(v))} />
          <label className="check">
            <input type="checkbox" checked={pages > 1 && facing} disabled={pages < 2} onChange={(e) => setFacing(e.currentTarget.checked)} />
            Facing pages
          </label>
        </div>
        <footer className="start-footer">
          <button type="button" className="primary" title={`${format}, ${plural(pages)}`} onClick={create}>
            Create
            <Keys keys="Enter" />
          </button>
        </footer>
      </div>
    </dialog>
  )
}
