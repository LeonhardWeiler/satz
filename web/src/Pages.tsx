import { useState, type DragEvent } from 'react'
import { createPortal } from 'react-dom'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { ContextMenu } from './ContextMenu'

type Menu = { id: string; x: number; y: number }

/** Figma's pages list: pages in order, the shown one current, with a context menu. */
export function Pages({ editor }: { editor: Editor }) {
  const pages = useEditor(editor, (e) => e.snapshot.pages)
  const current = useEditor(editor, (e) => e.page.id)
  const [menu, setMenu] = useState<Menu | null>(null)
  const [dragging, setDragging] = useState<string | null>(null)
  const [drop, setDrop] = useState<{ index: number; at: 'above' | 'below' } | null>(null)

  const add = () => editor.showPage(editor.apply({ type: 'addPage', after: current })[0])
  const duplicate = (id: string) => editor.showPage(editor.apply({ type: 'duplicatePage', id })[0])
  const remove = (id: string) => editor.apply({ type: 'deletePage', id })

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
        <h2>Pages</h2>
        <button type="button" className="icon-button" aria-label="Add page" title="Add page" onClick={add}>
          <Icon name="plus" size={16} />
        </button>
      </header>
      <ul className="tree page-list">
        {pages.map((p, i) => (
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
                onContextMenu={(e) => {
                  e.preventDefault()
                  setMenu({ id: p.id, x: e.clientX, y: e.clientY })
                }}
                onKeyDown={(e) => {
                  if ((e.key === 'Delete' || e.key === 'Backspace') && pages.length > 1) {
                    remove(p.id)
                    e.preventDefault()
                  }
                }}
              >
                Page {i + 1}
              </button>
            </div>
          </li>
        ))}
      </ul>
      {menu &&
        createPortal(
          <ContextMenu
            menu={menu}
            label="Page"
            onClose={() => setMenu(null)}
            items={[
              ['Duplicate page', () => duplicate(menu.id), true],
              ['Delete page', () => remove(menu.id), pages.length > 1],
            ]}
          />,
          document.body,
        )}
    </nav>
  )
}
