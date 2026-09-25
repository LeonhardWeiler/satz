import { useState, type DragEvent, type MouseEvent } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'

type Menu = { id: string; master: boolean; x: number; y: number }

/** The letter a page shows for its master: the prefix of "A-Master", or the first letter. */
export const prefix = (name: string) => (/^(\w{1,3})-/.exec(name)?.[1] ?? name.slice(0, 1)).toUpperCase()

/**
 * Figma's pages list with InDesign's masters above the pages: the shown page or master
 * is current, pages are dragged to reorder, both have a context menu.
 */
export function Pages({ editor }: { editor: Editor }) {
  const pages = useEditor(editor, (e) => e.snapshot.pages)
  const masters = useEditor(editor, (e) => e.snapshot.masters)
  const current = useEditor(editor, (e) => e.page.id)
  const [menu, setMenu] = useState<Menu | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [mastersOpen, setMastersOpen] = useState(true)
  const [dragging, setDragging] = useState<string | null>(null)
  const [drop, setDrop] = useState<{ index: number; at: 'above' | 'below' } | null>(null)

  const addPage = () => editor.showPage(editor.apply({ type: 'addPage', after: pages.some((p) => p.id === current) ? current : null })[0])
  const addMaster = () => {
    setMastersOpen(true)
    editor.showPage(editor.apply({ type: 'addMaster', like: current })[0])
  }
  const rename = (id: string, name: string | null) => {
    if (name) editor.apply({ type: 'setMaster', id, name })
    setRenaming(null)
  }
  const openMenu = (e: MouseEvent, id: string, master: boolean) => {
    e.preventDefault()
    setMenu({ id, master, x: e.clientX, y: e.clientY })
  }

  const over = (e: DragEvent, index: number) => {
    if (!dragging) return
    e.preventDefault()
    const r = e.currentTarget.getBoundingClientRect()
    const at = e.clientY < r.top + r.height / 2 ? 'above' : 'below'
    if (drop?.index !== index || drop.at !== at) setDrop({ index, at })
  }
  const onDrop = (e: DragEvent) => {
    e.preventDefault()
    if (dragging && drop) {
      const from = pages.findIndex((p) => p.id === dragging)
      const to = drop.index + (drop.at === 'below' ? 1 : 0)
      editor.apply({ type: 'movePage', id: dragging, index: to > from ? to - 1 : to })
    }
    setDrop(null)
  }

  return (
    <nav className="panel pages" aria-label="Pages" onPointerDown={editor.gesture}>
      <header className="panel-header">
        <button
          type="button"
          className="section-toggle"
          aria-expanded={mastersOpen}
          onClick={() => setMastersOpen(!mastersOpen)}
        >
          <Icon name="chevron" size={16} />
          <h2>Masters</h2>
        </button>
        <button type="button" className="icon-button" aria-label="Add master" title="Add master" onClick={addMaster}>
          <Icon name="plus" size={16} />
        </button>
      </header>
      {mastersOpen && masters.length > 0 && (
        <ul className="tree page-list" aria-label="Masters">
          {masters.map((m) => (
            <li key={m.id}>
              <div className="layer">
                {renaming === m.id ? (
                  <input
                    className="rename"
                    aria-label="Master name"
                    defaultValue={m.name}
                    autoFocus
                    onFocus={(e) => e.currentTarget.select()}
                    onBlur={(e) => rename(m.id, e.currentTarget.value.trim())}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') e.currentTarget.blur()
                      if (e.key === 'Escape') rename(m.id, null)
                    }}
                  />
                ) : (
                  <button
                    type="button"
                    className="layer-name page-name"
                    aria-current={m.id === current ? 'page' : undefined}
                    onClick={() => editor.showPage(m.id)}
                    onDoubleClick={() => setRenaming(m.id)}
                    onContextMenu={(e) => openMenu(e, m.id, true)}
                  >
                    {m.name}
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
      <header className="panel-header subheader">
        <h2>Pages</h2>
        <button type="button" className="icon-button" aria-label="Add page" title="Add page" onClick={addPage}>
          <Icon name="plus" size={16} />
        </button>
      </header>
      <ul className="tree page-list" aria-label="Pages">
        {pages.map((p, i) => {
          const master = masters.find((m) => m.id === p.master)
          return (
            <li key={p.id}>
              <div
                className="layer"
                data-drop={drop?.index === i ? drop.at : undefined}
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.effectAllowed = 'move'
                  e.dataTransfer.setData('text/plain', p.id)
                  setDragging(p.id)
                }}
                onDragOver={(e) => over(e, i)}
                onDragLeave={() => setDrop(null)}
                onDrop={onDrop}
                onDragEnd={() => {
                  setDragging(null)
                  setDrop(null)
                }}
              >
                <button
                  type="button"
                  className="layer-name page-name"
                  aria-current={p.id === current ? 'page' : undefined}
                  onClick={() => editor.showPage(p.id)}
                  onContextMenu={(e) => openMenu(e, p.id, false)}
                  onKeyDown={(e) => {
                    if ((e.key === 'Delete' || e.key === 'Backspace') && pages.length > 1) {
                      editor.apply({ type: 'deletePage', id: p.id })
                      e.preventDefault()
                    }
                  }}
                >
                  Page {i + 1}
                </button>
                {master && (
                  <span className="master-badge" title={`Master ${master.name}`}>
                    {prefix(master.name)}
                  </span>
                )}
              </div>
            </li>
          )
        })}
      </ul>
      {menu &&
        createPortal(
          <ContextMenu
            menu={menu}
            label={menu.master ? 'Master' : 'Page'}
            onClose={() => setMenu(null)}
            items={
              menu.master
                ? [
                    ['Rename master', () => setRenaming(menu.id), true],
                    ['Delete master', () => editor.apply({ type: 'deleteMaster', id: menu.id }), true],
                  ]
                : [
                    ['Duplicate page', () => editor.showPage(editor.apply({ type: 'duplicatePage', id: menu.id })[0]), true],
                    ['Delete page', () => editor.apply({ type: 'deletePage', id: menu.id }), pages.length > 1],
                  ]
            }
          />,
          document.body,
        )}
    </nav>
  )
}
