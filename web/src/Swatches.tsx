import { useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { fromRgb, neutral, resolve, rgb, type Color } from './color'
import { Chip, NO_SCOPE, Picker, SwatchOption } from './ColorPicker'
import { ContextMenu } from './ContextMenu'
import { Edge, NameInput, nextName } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'

export function Swatches({ editor }: { editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const swatches = snapshot.swatches
  const mode = useEditor(editor, (e) => e.snapshot.colorMode)
  const [editing, setEditing] = useState<string | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [open, setOpen] = useState(true)
  const [menu, setMenu] = useState<{ x: number; y: number; id: string } | null>(null)
  const list = useRef<HTMLDivElement>(null)
  const swatch = swatches.find((s) => s.id === editing)

  const add = () => {
    const node = editor.selected()[0]
    const fill = node?.fills.find((f) => f.type === 'solid')
    const color = fill ? resolve(fill.color, scopeOf(snapshot, node.activeModes)) : neutral('black', mode)
    const name = nextName('Swatch', swatches.map((s) => s.name))
    setEditing(editor.apply({ type: 'addSwatch', name, color, spot: false })[0])
  }
  const remove = (id: string) => {
    editor.apply({ type: 'deleteSwatch', id })
    if (editing === id) setEditing(null)
  }
  const rename = (id: string) => {
    setEditing(null)
    setRenaming(id)
  }
  const duplicate = (id: string) => {
    const s = swatches.find((s) => s.id === id)!
    editor.apply({ type: 'addSwatch', name: nextName(s.name.replace(/ \d+$/, ''), swatches.map((s) => s.name)), color: s.color, spot: s.spot })
  }
  const set = (patch: { name?: string; color?: Color; spot?: boolean }) =>
    editing && editor.apply({ type: 'setSwatch', id: editing, ...patch })

  return (
    <section className="panel swatches" aria-label="Swatches" onPointerDown={editor.gesture}>
      <Edge side="swatches" />
      <header className="panel-header">
        <button type="button" className="panel-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
          <span className="chevron" data-open={open || undefined}>
            <Icon name="chevron" />
          </span>
          <h2>Swatches</h2>
        </button>
        <button type="button" className="icon-button" aria-label="Add swatch" title="Add swatch" onClick={add}>
          <Icon name="plus" />
        </button>
      </header>
      <div ref={list} role="listbox" aria-label="Swatches" className="swatch-list" hidden={!open}>
        {swatches.map((s) => (
          <div
            key={s.id}
            className="swatch-row"
            data-id={s.id}
            onDoubleClick={() => rename(s.id)}
            onContextMenu={(e) => {
              e.preventDefault()
              setMenu({ x: e.clientX, y: e.clientY, id: s.id })
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && renaming !== s.id) setEditing(s.id)
            }}
          >
            {renaming === s.id ? (
              <div className="swatch-option" onBlur={() => setRenaming(null)}>
                <Chip color={s.color} scope={NO_SCOPE} />
                <NameInput label="Swatch name" value={s.name} autoFocus onCommit={(name) => editor.apply({ type: 'setSwatch', id: s.id, name })} />
              </div>
            ) : (
              <SwatchOption swatch={s} selected={s.id === editing} onPick={() => setEditing(s.id)} />
            )}
            {renaming !== s.id && (
              <button
                type="button"
                className="swatch-delete"
                aria-label={`Delete ${s.name}`}
                title="Delete swatch"
                onClick={() => remove(s.id)}
              >
                <Icon name="minus" />
              </button>
            )}
          </div>
        ))}
      </div>
      {menu &&
        createPortal(
          <ContextMenu
            anchor={() => new DOMRect(menu.x, menu.y)}
            label="Swatch"
            onClose={() => setMenu(null)}
            items={[
              ['Edit', () => setEditing(menu.id), true],
              ['Rename', () => rename(menu.id), true],
              ['Duplicate', () => duplicate(menu.id), true],
              ['Delete', () => remove(menu.id), true],
            ]}
          />,
          document.body,
        )}
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
            scope={scopeOf(snapshot)}
            onChange={(color) => set({ color })}
            onClose={() => setEditing(null)}
          >
            <div className="grid">
              <label className="field">
                <span className="field-label">Name</span>
                <NameInput label="Name" value={swatch.name} onCommit={(name) => set({ name })} />
              </label>
              <label className="check">
                <input
                  type="checkbox"
                  checked={swatch.spot}
                  onChange={(e) => {
                    const spot = e.currentTarget.checked
                    const c = swatch.color
                    set({ spot, color: spot && typeof c === 'number' ? fromRgb(rgb(c, scopeOf(snapshot)), c, 'cmyk') : c })
                  }}
                />
                Spot color
              </label>
            </div>
          </Picker>,
          document.body,
        )}
    </section>
  )
}
