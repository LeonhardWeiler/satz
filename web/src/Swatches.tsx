import { useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { fromRgb, neutral, resolve, rgb, type Color } from './color'
import { Picker, SwatchOption } from './ColorPicker'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'

export function Swatches({ editor }: { editor: Editor }) {
  const swatches = useEditor(editor, (e) => e.snapshot.swatches)
  const mode = useEditor(editor, (e) => e.snapshot.colorMode)
  const [editing, setEditing] = useState<string | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const swatch = swatches.find((s) => s.id === editing)

  const add = () => {
    const fill = editor.selected()[0]?.fills.find((f) => f.type === 'solid')
    const color = fill ? resolve(fill.color, swatches) : neutral('black', mode)
    setEditing(editor.apply({ type: 'addSwatch', name: `Swatch ${swatches.length + 1}`, color, spot: false })[0])
  }
  const set = (patch: { name?: string; color?: Color; spot?: boolean }) =>
    editing && editor.apply({ type: 'setSwatch', id: editing, ...patch })

  return (
    <section className="panel swatches" aria-label="Swatches" onPointerDown={editor.gesture}>
      <header className="panel-header">
        <h2>Swatches</h2>
        <button type="button" className="icon-button" aria-label="Add swatch" title="Add swatch" onClick={add}>
          <Icon name="plus" size={16} />
        </button>
      </header>
      <div ref={list} role="listbox" aria-label="Swatches" className="swatch-list">
        {swatches.map((s) => (
          <div
            key={s.id}
            data-id={s.id}
            onDoubleClick={() => setEditing(s.id)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') setEditing(s.id)
            }}
          >
            <SwatchOption swatch={s} selected={s.id === editing} onPick={() => {}} />
          </div>
        ))}
      </div>
      {swatch &&
        createPortal(
          <Picker
            key={swatch.id}
            anchor={() => {
              const row = list.current!.querySelector(`[data-id="${CSS.escape(swatch.id)}"]`) ?? list.current!
              const panel = list.current!.closest('.panel')!.getBoundingClientRect()
              const r = row.getBoundingClientRect()
              return new DOMRect(panel.x, r.y, panel.width, r.height)
            }}
            side="right"
            label="Edit swatch"
            title="Edit swatch"
            color={swatch.color}
            mode={swatch.spot ? 'cmyk' : mode}
            swatches={swatches}
            onChange={(color) => set({ color })}
            onClose={() => setEditing(null)}
            footer={
              <button
                type="button"
                className="button"
                onClick={() => {
                  editor.apply({ type: 'deleteSwatch', id: swatch.id })
                  setEditing(null)
                }}
              >
                Delete swatch
              </button>
            }
          >
            <label className="field">
              <span className="field-label">Name</span>
              <input
                name="swatch-name"
                aria-label="Name"
                autoComplete="off"
                spellCheck={false}
                defaultValue={swatch.name}
                onBlur={(e) => {
                  const name = e.currentTarget.value.trim()
                  if (name && name !== swatch.name) set({ name })
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') e.currentTarget.blur()
                }}
              />
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={swatch.spot}
                onChange={(e) => {
                  const spot = e.currentTarget.checked
                  const c = swatch.color
                  set({ spot, color: spot && typeof c === 'number' ? fromRgb(rgb(c, swatches), c, 'cmyk') : c })
                }}
              />
              Spot color
            </label>
          </Picker>,
          document.body,
        )}
    </section>
  )
}
