import { useEffect, useRef, useState, type ReactNode } from 'react'
import { ColorPicker } from './ColorPicker'
import { neutral } from './color'
import { Field, NameInput, nextName, Select } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import type { Bindable as Prop, Modes, Value } from './model'
import { Popover } from './Popover'

/** The local variables dialog: collections on the left, a variable per row and a mode per column. */
export function Variables({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const ref = useRef<HTMLDialogElement>(null)
  const create = useRef<HTMLButtonElement>(null)
  const [selected, setSelected] = useState<string | null>(null)
  const [menu, setMenu] = useState(false)
  const { collections, colorMode } = snapshot
  const collection = collections.find((c) => c.id === selected) ?? collections[0]
  const variables = snapshot.variables.filter((v) => v.collection === collection?.id)
  const scope = { ...scopeOf(snapshot), variables: [] }

  useEffect(() => ref.current!.showModal(), [])

  const addVariable = (kind: 'color' | 'number') => {
    setMenu(false)
    const prefix = kind === 'color' ? 'Color' : 'Number'
    const value: Value = kind === 'color' ? { color: neutral('black', colorMode) } : { number: 0 }
    const name = nextName(prefix, variables.map((v) => v.name))
    editor.apply({ type: 'addVariable', collection: collection!.id, name, value })
  }

  return (
    <dialog ref={ref} className="variables" aria-label="Local variables" onClose={onClose} onPointerDown={editor.gesture}>
      <header className="panel-header">
        <h2>Local variables</h2>
        <button type="button" className="icon-button" aria-label="Close" title="Close" onClick={() => ref.current!.close()}>
          <Icon name="close" size={16} />
        </button>
      </header>
      <div className="variables-body">
        <nav aria-label="Collections" className="collections">
          <header className="section-header">
            <h3>Collections</h3>
            <button
              type="button"
              className="icon-button"
              aria-label="Create collection"
              title="Create collection"
              onClick={() => {
                const name = nextName('Collection', collections.map((c) => c.name))
                setSelected(editor.apply({ type: 'addCollection', name })[0])
              }}
            >
              <Icon name="plus" size={16} />
            </button>
          </header>
          {collections.map((c) => (
            <button
              key={c.id}
              type="button"
              className="collection"
              aria-current={c.id === collection?.id}
              onClick={() => setSelected(c.id)}
            >
              {c.name}
            </button>
          ))}
        </nav>
        {collection ? (
          <div className="variables-main">
            <div className="row">
              <NameInput
                label="Collection name"
                value={collection.name}
                onCommit={(name) => editor.apply({ type: 'setCollection', id: collection.id, name })}
              />
              <button type="button" className="button" onClick={() => editor.apply({ type: 'deleteCollection', id: collection.id })}>
                Delete collection
              </button>
            </div>
            <div className="variables-table">
              <table>
                <thead>
                  <tr>
                    <th scope="col">Name</th>
                    {collection.modes.map((m) => (
                      <th key={m.id} scope="col">
                        <div className="row">
                          <NameInput
                            label="Mode name"
                            value={m.name}
                            onCommit={(name) => editor.apply({ type: 'setMode', collection: collection.id, id: m.id, name })}
                          />
                          {collection.modes.length > 1 && (
                            <button
                              type="button"
                              className="icon-button"
                              aria-label={`Delete mode ${m.name}`}
                              title={`Delete mode ${m.name}`}
                              onClick={() => editor.apply({ type: 'deleteMode', collection: collection.id, id: m.id })}
                            >
                              <Icon name="minus" size={16} />
                            </button>
                          )}
                        </div>
                      </th>
                    ))}
                    <th>
                      <button
                        type="button"
                        className="icon-button"
                        aria-label="Add mode"
                        title="Add mode"
                        onClick={() =>
                          editor.apply({
                            type: 'addMode',
                            collection: collection.id,
                            name: nextName('Mode', collection.modes.map((m) => m.name)),
                          })
                        }
                      >
                        <Icon name="plus" size={16} />
                      </button>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {variables.map((v) => (
                    <tr key={v.id}>
                      <th scope="row">
                        <NameInput label="Variable name" value={v.name} onCommit={(name) => editor.apply({ type: 'setVariable', id: v.id, name })} />
                      </th>
                      {collection.modes.map((m) => {
                        const value = v.values[m.id]
                        const label = `${v.name} in ${m.name}`
                        const set = (value: Value) => editor.apply({ type: 'setVariable', id: v.id, mode: m.id, value })
                        return (
                          <td key={m.id}>
                            {'color' in value ? (
                              <div className="row">
                                <ColorPicker label={label} color={value.color} mode={colorMode} scope={scope} bindable onChange={(color) => set({ color })} />
                              </div>
                            ) : (
                              <Field label="" title={label} unit="" value={value.number} onCommit={(number) => set({ number })} />
                            )}
                          </td>
                        )
                      })}
                      <td>
                        <button
                          type="button"
                          className="icon-button"
                          aria-label={`Delete variable ${v.name}`}
                          title={`Delete variable ${v.name}`}
                          onClick={() => editor.apply({ type: 'deleteVariable', id: v.id })}
                        >
                          <Icon name="minus" size={16} />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div
              className="create-variable"
              onBlur={(e) => {
                if (!e.currentTarget.contains(e.relatedTarget)) setMenu(false)
              }}
              onKeyDown={(e) => {
                if (e.key === 'Escape' && menu) {
                  e.preventDefault()
                  setMenu(false)
                }
              }}
            >
              <button
                ref={create}
                type="button"
                className="button"
                aria-haspopup="menu"
                aria-expanded={menu}
                onClick={() => setMenu((o) => !o)}
              >
                <Icon name="plus" size={16} />
                Create variable
              </button>
              {menu && (
                <Popover anchor={() => create.current!.getBoundingClientRect()} side="top" className="menu" role="menu" aria-label="Variable type">
                  {(['color', 'number'] as const).map((kind) => (
                    <button key={kind} type="button" role="menuitem" className="menu-item" onClick={() => addVariable(kind)}>
                      <span>{kind === 'color' ? 'Color' : 'Number'}</span>
                    </button>
                  ))}
                </Popover>
              )}
            </div>
          </div>
        ) : (
          <p className="empty">No collections yet. Create one to add variables.</p>
        )}
      </div>
    </dialog>
  )
}

/** A mode select per collection with several modes for the page or layer `id`. */
export function ModeSelects({ editor, id, own, inherited }: { editor: Editor; id: string; own: Modes; inherited: Modes }) {
  const collections = useEditor(editor, (e) => e.snapshot.collections)
  return collections
    .filter((c) => c.modes.length > 1)
    .map((c) => {
      const auto = c.modes.find((m) => m.id === inherited[c.id]) ?? c.modes[0]
      const options = Object.fromEntries([['', `Auto (${auto.name})`], ...c.modes.map((m) => [m.id, m.name])])
      return (
        <Select
          key={c.id}
          label={`${c.name} mode`}
          value={c.modes.some((m) => m.id === own[c.id]) ? own[c.id] : ''}
          options={options}
          onChange={(mode) => editor.apply({ type: 'useMode', id, collection: c.id, mode: mode || null })}
        />
      )
    })
}

/** Wraps the field `children` of `prop` on layer `id` with a button that binds a number variable to it. */
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
  label: string
  children: ReactNode
}) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const bound = (snapshot.textStyles.find((s) => s.id === id)?.bindings ?? editor.nodes.get(id)?.node.bindings)?.[prop]
  const variable = snapshot.variables.find((v) => v.id === bound)
  const numbers = snapshot.variables.filter((v) => 'number' in Object.values(v.values)[0])
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
            <Icon name="detach" size={16} />
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
            <Icon name="variable" size={16} />
          </button>
        </>
      )}
      {open && (
        <Popover anchor={() => ref.current!.getBoundingClientRect()} side="left" className="menu" role="listbox" aria-label="Number variables">
          {numbers.length === 0 && <p className="empty">No number variables yet. Add them under Local variables.</p>}
          {snapshot.collections.map((c) => {
            const vars = numbers.filter((v) => v.collection === c.id)
            return (
              vars.length > 0 && (
                <div key={c.id} role="group" aria-label={c.name} className="swatch-group">
                  <h3>{c.name}</h3>
                  {vars.map((v) => (
                    <button key={v.id} type="button" role="option" aria-selected={v.id === bound} className="menu-item" onClick={() => bind(v.id)}>
                      <span>{v.name}</span>
                    </button>
                  ))}
                </div>
              )
            )
          })}
        </Popover>
      )}
    </div>
  )
}
