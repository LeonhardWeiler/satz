import { useState, type DragEvent, type ReactNode } from 'react'
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

  const over = (e: DragEvent, node: Node) => {
    if (!dragging.length || dragging.includes(node.id)) return
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
          aria-level={level}
          aria-selected={selection.includes(node.id)}
          aria-expanded={kids ? open : undefined}
        >
          <div
            className="layer"
            style={{ paddingLeft: 4 + (level - 1) * 16 }}
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
                aria-label={open ? 'Collapse' : 'Expand'}
                data-open={open || undefined}
                onClick={() => toggle(node.id)}
              >
                <Icon name="chevron" size={16} />
              </button>
            ) : (
              <span className="chevron" />
            )}
            {mask && (
              <span className="kind masked" title={`Masked by ${mask.name}`}>
                <Icon name="masked" size={16} />
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
                onClick={(e) => select(node.id, e.shiftKey || e.ctrlKey || e.metaKey)}
                onDoubleClick={() => editor.set({ renaming: node.id })}
              >
                {node.name}
              </button>
            )}
          </div>
          {open && <ul role="group">{rows(node.children, level + 1)}</ul>}
        </li>
      )
    })

  return (
    <nav className="panel layers" aria-label="Layers">
      <header className="panel-header">
        <h2>Layers</h2>
      </header>
      <ul role="tree" aria-label="Layers" aria-multiselectable="true" className="tree">
        {rows(page.children, 1)}
      </ul>
    </nav>
  )
}
