import { useState, type DragEvent, type KeyboardEvent, type ReactNode } from 'react'
import { roam } from './controls'
import { useEditor, type Editor } from './editor'
import { Icon, KindIcon } from './icons'
import type { Node } from './model'

type Drop = { id: string; at: 'above' | 'below' | 'into' }

export function Layers({ editor }: { editor: Editor }) {
  const page = useEditor(editor, (e) => e.page)
  const selection = useEditor(editor, (e) => e.selection)
  const renaming = useEditor(editor, (e) => e.renaming)
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(new Set())
  const [drop, setDrop] = useState<Drop | null>(null)
  const [dragging, setDragging] = useState<string[]>([])

  const toggle = (id: string) =>
    setCollapsed((c) => {
      const next = new Set(c)
      if (!next.delete(id)) next.add(id)
      return next
    })

  const select = (id: string, add: boolean) =>
    editor.set({
      selection: add ? (selection.includes(id) ? selection.filter((s) => s !== id) : [...selection, id]) : [id],
    })

  const rename = (id: string, name: string | null) => {
    if (name) editor.apply({ type: 'set', id, name })
    editor.set({ renaming: null })
  }

  const shown = (nodes: Node[]): Node[] =>
    nodes.toReversed().flatMap((n) => [n, ...('children' in n && !collapsed.has(n.id) ? shown(n.children) : [])])
  const stop = shown(page.children).find((n) => selection.includes(n.id))?.id ?? page.children.at(-1)?.id

  const onKey = (e: KeyboardEvent<HTMLUListElement>) => {
    const items = [...e.currentTarget.querySelectorAll('.layer-name')]
    const item = (e.target as HTMLElement).closest<HTMLElement>('[role=treeitem]')
    if (roam(e, items) || !item || !items.includes(e.target as Element)) return
    const id = item.dataset.id!
    const entry = editor.nodes.get(id)
    const open = item.getAttribute('aria-expanded')
    const focus = (el: Element | null | undefined) => (el as HTMLElement | null)?.focus()
    if (e.key === 'Enter' || e.key === 'F2') e.stopPropagation()
    if (e.key === 'F2') editor.set({ renaming: id })
    else if (e.key === 'Enter' && e.shiftKey && entry?.parent) editor.set({ selection: [entry.parent.id] })
    else if (e.key === 'Enter' && selection.includes(id) && entry && 'children' in entry.node && entry.node.children.length) {
      editor.set({ selection: entry.node.children.map((c) => c.id) })
    } else if (e.key === 'ArrowRight' && open === 'false') toggle(item.dataset.id!)
    else if (e.key === 'ArrowRight' && open === 'true') focus(item.querySelector('[role=group] .layer-name'))
    else if (e.key === 'ArrowLeft' && open === 'true') toggle(item.dataset.id!)
    else if (e.key === 'ArrowLeft') focus(item.parentElement?.closest('[role=treeitem]')?.querySelector('.layer-name'))
    else return
    e.preventDefault()
  }

  const dragged = (id?: string): boolean => id !== undefined && (dragging.includes(id) || dragged(editor.nodes.get(id)?.parent?.id))

  const over = (e: DragEvent, node: Node) => {
    if (!dragging.length || dragged(node.id)) return
    e.preventDefault()
    const r = e.currentTarget.getBoundingClientRect()
    const f = (e.clientY - r.top) / r.height
    const container = node.kind === 'group' || node.kind === 'frame'
    const at = container && f > 0.25 && f < 0.75 ? 'into' : f < 0.5 ? 'above' : 'below'
    if (drop?.id !== node.id || drop.at !== at) setDrop({ id: node.id, at })
  }

  const onDrop = (e: DragEvent) => {
    e.preventDefault()
    if (!drop) return
    const { id, at } = drop
    const target = editor.nodes.get(id)!
    if (at === 'into' && 'children' in target.node) {
      editor.apply({ type: 'move', ids: dragging, parent: id, index: target.node.children.length })
    } else {
      const parent = target.parent
      const others = (parent?.children ?? page.children).filter((n) => !dragging.includes(n.id))
      const i = others.findIndex((n) => n.id === id)
      editor.apply({
        type: 'move',
        ids: dragging,
        parent: parent?.id ?? page.id,
        index: at === 'above' ? i + 1 : i,
      })
    }
    setDrop(null)
  }

  const rows = (nodes: Node[], level: number): ReactNode =>
    nodes.toReversed().map((node) => {
      const mask = nodes.slice(0, nodes.indexOf(node)).findLast((n) => n.mask)
      const kids = 'children' in node && node.children.length > 0
      const open = kids && !collapsed.has(node.id)
      return (
        <li
          key={node.id}
          role="treeitem"
          data-id={node.id}
          aria-level={level}
          aria-selected={selection.includes(node.id)}
          aria-expanded={kids ? open : undefined}
        >
          <div
            className="layer"
            style={{ paddingLeft: 4 + (level - 1) * 16 }}
            data-hidden={node.hidden || undefined}
            data-locked={node.locked || undefined}
            onMouseEnter={() => editor.set({ hover: node.id })}
            data-drop={drop?.id === node.id ? drop.at : undefined}
            draggable={renaming !== node.id}
            onDragStart={(e) => {
              const ids = selection.includes(node.id) ? selection : [node.id]
              e.dataTransfer.effectAllowed = 'move'
              e.dataTransfer.setData('text/plain', ids.join(' '))
              setDragging(ids)
            }}
            onDragOver={(e) => over(e, node)}
            onDragLeave={() => setDrop(null)}
            onDrop={onDrop}
            onDragEnd={() => {
              setDragging([])
              setDrop(null)
            }}
          >
            {kids ? (
              <button
                type="button"
                className="chevron"
                tabIndex={-1}
                aria-label={open ? 'Collapse' : 'Expand'}
                data-open={open || undefined}
                onClick={() => toggle(node.id)}
              >
                <Icon name="chevron" />
              </button>
            ) : (
              <span className="chevron" />
            )}
            {mask && (
              <span className="kind masked" title={`Masked by ${mask.name}`}>
                <Icon name="masked" />
              </span>
            )}
            <KindIcon node={node} />
            {renaming === node.id ? (
              <input
                className="rename"
                aria-label="Layer name"
                defaultValue={node.name}
                autoFocus
                onFocus={(e) => e.currentTarget.select()}
                onBlur={(e) => rename(node.id, e.currentTarget.value.trim())}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') e.currentTarget.blur()
                  if (e.key === 'Escape') rename(node.id, null)
                }}
              />
            ) : (
              <button
                type="button"
                className="layer-name"
                tabIndex={node.id === stop ? 0 : -1}
                onClick={(e) => select(node.id, e.shiftKey || e.ctrlKey || e.metaKey)}
                onDoubleClick={() => editor.set({ renaming: node.id })}
              >
                {node.name}
              </button>
            )}
            <span className="acts">
              <button
                type="button"
                tabIndex={-1}
                aria-label={node.locked ? 'Unlock' : 'Lock'}
                title={`${node.locked ? 'Unlock' : 'Lock'} (Ctrl+Shift+L)`}
                onClick={() => editor.apply({ type: 'set', id: node.id, locked: !node.locked })}
              >
                <Icon name={node.locked ? 'lock' : 'unlock'} />
              </button>
              <button
                type="button"
                tabIndex={-1}
                aria-label={node.hidden ? 'Show' : 'Hide'}
                title={`${node.hidden ? 'Show' : 'Hide'} (Ctrl+Shift+H)`}
                onClick={() => editor.apply({ type: 'set', id: node.id, hidden: !node.hidden })}
              >
                <Icon name={node.hidden ? 'eyeOff' : 'eye'} />
              </button>
            </span>
          </div>
          {open && <ul role="group">{rows(node.children, level + 1)}</ul>}
        </li>
      )
    })

  const master = editor.masterOf(page)
  return (
    <nav className="panel layers" aria-label="Layers">
      <header className="panel-header">
        <h2>Layers</h2>
      </header>
      <ul
        role="tree"
        aria-label="Layers"
        aria-multiselectable="true"
        className="tree"
        onKeyDown={onKey}
        onMouseLeave={() => editor.set({ hover: null })}
      >
        {rows(page.children, 1)}
      </ul>
      {!page.children.length && <p className="tree-note">Draw with R, O or T</p>}
      {master && (
        <p className="tree-note master-row">
          <Icon name="master" />
          {master.name} master items
        </p>
      )}
    </nav>
  )
}
