import { useEffect, useRef, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { Chip, ColorPicker } from './ColorPicker'
import { ContextMenu } from './ContextMenu'
import { neutral } from './color'
import { Field, FontSelect, NameInput, nextName, Section, Select } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import type { Bindable as Prop, Modes, Typeface, Value } from './model'
import { Popover } from './Popover'

type Kind = 'color' | 'number' | 'font'
const KINDS: Record<Kind, string> = { color: 'Color', number: 'Number', font: 'Font' }
const kindOf = (v: Value): Kind => ('color' in v ? 'color' : 'number' in v ? 'number' : 'font')

/** The document's variables: the plus makes one, a click edits it in a popover; like swatches, a right click or double click renames. */
export function Variables({ editor }: { editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [menu, setMenu] = useState<DOMRect | null>(null)
  const [open, setOpen] = useState<string | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [rowMenu, setRowMenu] = useState<{ x: number; y: number; id: string } | null>(null)
  const { collections, variables, colorMode } = snapshot
  const scope = { ...scopeOf(snapshot), variables: [] }
  const fonts = [...snapshot.fonts, ...snapshot.missingFonts.map((m) => m.font)]

  const add = (kind: Kind) =>
    editor.batch(() => {
      const collection = collections[0]?.id ?? editor.apply({ type: 'addCollection', name: 'Variables' })[0]
      const value: Value = kind === 'color' ? { color: neutral('black', colorMode) } : kind === 'number' ? { number: 0 } : { font: null }
      const name = nextName(KINDS[kind], variables.map((v) => v.name))
      setOpen(editor.apply({ type: 'addVariable', collection, name, value })[0] ?? null)
    })
  const remove = (id: string) => {
    editor.apply({ type: 'deleteVariable', id })
    if (open === id) setOpen(null)
  }
  const rename = (id: string) => {
    setOpen(null)
    setRenaming(id)
  }
  const duplicate = (id: string) =>
    editor.batch(() => {
      const v = variables.find((v) => v.id === id)!
      const [mode, ...modes] = collections.find((c) => c.id === v.collection)!.modes
      const name = nextName(v.name.replace(/ \d+$/, ''), variables.map((v) => v.name))
      const [copy] = editor.apply({ type: 'addVariable', collection: v.collection, name, value: v.values[mode.id] })
      if (copy) for (const m of modes) editor.apply({ type: 'setVariable', id: copy, mode: m.id, value: v.values[m.id] })
    })
  const preview = (v: Value) =>
    'color' in v ? <Chip color={v.color} scope={scope} /> : 'number' in v ? <span>{v.number}</span> : <span>{v.font?.name ?? fonts[0].name}</span>

  return (
    <Section title="Variables" onAdd={(e) => setMenu(e.currentTarget.getBoundingClientRect())}>
      {variables.length > 0 && (
        <div className="var-group">
          {variables.map((v) => (
            <div
              key={v.id}
              className="swatch-row"
              data-variable={v.id}
              onDoubleClick={() => rename(v.id)}
              onContextMenu={(e) => {
                e.preventDefault()
                setRowMenu({ x: e.clientX, y: e.clientY, id: v.id })
              }}
            >
              {renaming === v.id ? (
                <div className="var" onBlur={() => setRenaming(null)}>
                  <Icon name={kindOf(Object.values(v.values)[0]) === 'font' ? 'text' : 'variable'} />
                  <NameInput label="Variable name" value={v.name} autoFocus onCommit={(name) => editor.apply({ type: 'setVariable', id: v.id, name })} />
                </div>
              ) : (
                <>
                  <button type="button" className="var" aria-haspopup="dialog" aria-expanded={open === v.id} onClick={() => setOpen(open === v.id ? null : v.id)}>
                    <Icon name={kindOf(Object.values(v.values)[0]) === 'font' ? 'text' : 'variable'} />
                    <span className="var-name">{v.name}</span>
                    <span className="var-value">{preview(v.values[collections.find((c) => c.id === v.collection)!.modes[0].id])}</span>
                  </button>
                  <button type="button" className="swatch-delete" aria-label={`Delete ${v.name}`} title="Delete variable" onClick={() => remove(v.id)}>
                    <Icon name="minus" />
                  </button>
                </>
              )}
            </div>
          ))}
        </div>
      )}
      {rowMenu &&
        createPortal(
          <ContextMenu
            anchor={() => new DOMRect(rowMenu.x, rowMenu.y)}
            label="Variable"
            onClose={() => setRowMenu(null)}
            items={[
              ['Edit', () => setOpen(rowMenu.id), true],
              ['Rename', () => rename(rowMenu.id), true],
              ['Duplicate', () => duplicate(rowMenu.id), true],
              ['Delete', () => remove(rowMenu.id), true],
            ]}
          />,
          document.body,
        )}
      {menu &&
        createPortal(
          <ContextMenu
            anchor={() => menu}
            side="bottom"
            label="Create variable"
            onClose={() => setMenu(null)}
            items={(Object.keys(KINDS) as Kind[]).map((k): [string, () => void, boolean] => [KINDS[k], () => add(k), true])}
          />,
          document.body,
        )}
      {open &&
        variables.some((v) => v.id === open) &&
        createPortal(<VariableEditor editor={editor} id={open} fonts={fonts} onClose={() => setOpen(null)} />, document.body)}
    </Section>
  )
}

/** Name and value per mode of the variable `id`, beside its row. */
function VariableEditor({ editor, id, fonts, onClose }: { editor: Editor; id: string; fonts: Typeface[]; onClose: () => void }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const ref = useRef<HTMLDivElement>(null)
  const v = snapshot.variables.find((v) => v.id === id)!
  const c = snapshot.collections.find((c) => c.id === v.collection)!
  const scope = { ...scopeOf(snapshot), variables: [] }
  const row = () => document.querySelector(`[data-variable="${id}"]`)

  useEffect(() => ref.current?.focus(), [])
  useEffect(() => {
    const aside = (t: EventTarget | null) => t instanceof Element && !!t.closest('.picker:not(.var-editor), .menu-backdrop')
    const down = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node) && !row()?.contains(e.target as Node) && !aside(e.target)) onClose()
    }
    const key = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented || aside(document.activeElement)) return
      e.preventDefault()
      e.stopPropagation()
      onClose()
    }
    document.addEventListener('pointerdown', down, true)
    document.addEventListener('keydown', key, true)
    return () => {
      document.removeEventListener('pointerdown', down, true)
      document.removeEventListener('keydown', key, true)
    }
  })

  return (
    <Popover
      ref={ref}
      anchor={() => {
        const b = row()?.getBoundingClientRect() ?? new DOMRect()
        const panel = row()?.closest('.panel')?.getBoundingClientRect() ?? b
        return new DOMRect(panel.x, b.y, panel.width, b.height)
      }}
      side="left"
      className="picker var-editor"
      role="dialog"
      aria-label={`Edit ${v.name}`}
      tabIndex={-1}
    >
      <header className="picker-header">
        <h2>Edit variable</h2>
        <button type="button" className="icon-button" aria-label="Close" title="Close" onClick={onClose}>
          <Icon name="close" />
        </button>
      </header>
      <NameInput label="Variable name" value={v.name} onCommit={(name) => editor.apply({ type: 'setVariable', id, name })} />
      {c.modes.map((m) => {
        const value = v.values[m.id]
        const label = `${v.name} in ${m.name}`
        const set = (value: Value) => editor.apply({ type: 'setVariable', id, mode: m.id, value })
        return (
          <div key={m.id} className="var-mode">
            <NameInput label="Mode name" value={m.name} onCommit={(name) => editor.apply({ type: 'setMode', collection: c.id, id: m.id, name })} />
            {'color' in value ? (
              <ColorPicker label={label} color={value.color} mode={snapshot.colorMode} scope={scope} bindable onChange={(color) => set({ color })} />
            ) : 'number' in value ? (
              <Field label="" title={label} unit="" min={-Infinity} value={value.number} onCommit={(number) => set({ number })} />
            ) : (
              <FontSelect label={label} faces={fonts} value={(value.font ?? fonts[0]).hash} onChange={(font) => set({ font })} />
            )}
            {c.modes.length > 1 && (
              <button
                type="button"
                className="icon-button"
                aria-label={`Delete mode ${m.name}`}
                title={`Delete mode ${m.name}`}
                onClick={() => editor.apply({ type: 'deleteMode', collection: c.id, id: m.id })}
              >
                <Icon name="minus" />
              </button>
            )}
          </div>
        )
      })}
      <div className="row">
        <button
          type="button"
          className="button"
          onClick={() => editor.apply({ type: 'addMode', collection: c.id, name: nextName('Mode', c.modes.map((m) => m.name)) })}
        >
          <Icon name="plus" />
          Add mode
        </button>
        <button
          type="button"
          className="button"
          onClick={() => {
            onClose()
            editor.apply({ type: 'deleteVariable', id })
          }}
        >
          Delete variable
        </button>
      </div>
    </Popover>
  )
}

/** A mode select per collection with several modes for the page or layer `id`. */
export function ModeSelects({ editor, id, own, inherited }: { editor: Editor; id: string; own: Modes; inherited: Modes }) {
  const collections = useEditor(editor, (e) => e.snapshot.collections)
  return collections
    .filter((c) => c.modes.length > 1)
    .map((c) => {
      const auto = c.modes.find((m) => m.id === inherited[c.id]) ?? c.modes[0]
      const prefix = collections.length > 1 ? `${c.name}: ` : ''
      const options = Object.fromEntries([['', `${prefix}auto (${auto.name})`], ...c.modes.map((m) => [m.id, `${prefix}${m.name}`])])
      return (
        <Select
          key={c.id}
          label={prefix ? `${c.name} mode` : 'Mode'}
          value={c.modes.some((m) => m.id === own[c.id]) ? own[c.id] : ''}
          options={options}
          onChange={(mode) => editor.apply({ type: 'useMode', id, collection: c.id, mode: mode || null })}
        />
      )
    })
}

/** Wraps the field `children` of `prop` on layer `id` with a button that binds a number or font variable to it. */
export function Bindable({
  editor,
  id,
  prop,
  title,
  label,
  children,
}: {
  editor: Editor
  id: string
  prop: Prop
  title: string
  label: ReactNode
  children: ReactNode
}) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const bound = (snapshot.textStyles.find((s) => s.id === id)?.bindings ?? editor.nodes.get(id)?.node.bindings)?.[prop]
  const variable = snapshot.variables.find((v) => v.id === bound)
  const kind = prop === 'font' ? 'font' : 'number'
  const numbers = snapshot.variables.filter((v) => kindOf(Object.values(v.values)[0]) === kind)
  const bind = (variable: string | null) => {
    setOpen(false)
    editor.apply({ type: 'bind', id, prop, variable })
  }

  return (
    <div
      ref={ref}
      className="bindable"
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) setOpen(false)
      }}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && open) {
          e.stopPropagation()
          setOpen(false)
        }
      }}
    >
      {variable ? (
        <div className="field">
          {label && <span className="field-label">{label}</span>}
          <button
            type="button"
            className="pill"
            aria-label={`${title}: ${variable.name}`}
            title={`${title}: ${variable.name}`}
            aria-haspopup="listbox"
            aria-expanded={open}
            onClick={() => setOpen((o) => !o)}
          >
            {variable.name}
          </button>
          <button
            type="button"
            className="icon-button"
            aria-label={`Detach variable from ${title}`}
            title="Detach variable"
            onClick={() => bind(null)}
          >
            <Icon name="detach" />
          </button>
        </div>
      ) : (
        <>
          {children}
          <button
            type="button"
            className="icon-button bind-button"
            aria-label={`Apply variable to ${title}`}
            title="Apply variable"
            aria-haspopup="listbox"
            aria-expanded={open}
            onClick={() => setOpen((o) => !o)}
          >
            <Icon name="variable" />
          </button>
        </>
      )}
      {open && (
        <Popover anchor={() => ref.current!.getBoundingClientRect()} side="left" className="menu" role="listbox" aria-label={`${KINDS[kind]} variables`}>
          {numbers.length === 0 && <p className="menu-empty">No {kind} variables yet. Add them under Variables.</p>}
          {numbers.map((v) => (
            <button key={v.id} type="button" role="option" aria-selected={v.id === bound} className="menu-item" onClick={() => bind(v.id)}>
              <span>{v.name}</span>
            </button>
          ))}
        </Popover>
      )}
    </div>
  )
}
