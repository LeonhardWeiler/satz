import { useEffect, useRef, useState, type DragEvent, type KeyboardEvent, type MouseEvent } from 'react'
import type { CanvasKit } from 'canvaskit-wasm'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { roam } from './controls'
import { prefix } from './engine/engine'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'
import type { Page, Snapshot } from './model'
import { Renderer, type Sheet } from './renderer'

type Lists = { id: string; x: number }[]
type Menu = { id: string; master: boolean; x: number; y: number }

const PAGE = 150
const MASTER = 110

/** Draws pictures of pages with the canvas renderer, as many per frame as fit in 8 ms. */
function usePaint(ck: CanvasKit, editor: Editor) {
  const [painter] = useState(() => {
    let renderer: Renderer | undefined
    const queue = new Map<HTMLCanvasElement, [Lists, Sheet[]]>()
    let frame = 0
    const draw = (c: HTMLCanvasElement, lists: Lists, sheets: Sheet[]) => {
      if (!c.isConnected) return
      const left = Math.min(...sheets.map((s) => s.x))
      const width = Math.round(c.clientWidth * devicePixelRatio)
      const height = Math.round(c.clientHeight * devicePixelRatio)
      const zoom = width / (Math.max(...sheets.map((s) => s.x + s.width)) - left)
      const surface = ck.MakeSurface(width, height)
      if (!surface) return
      renderer ??= new Renderer(ck, editor.engine)
      renderer.draw(surface.getCanvas(), lists, sheets, { x: -left * zoom, y: 0, zoom }, 1, { selection: [] })
      const info = { width, height, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB }
      const px = surface.getCanvas().readPixels(0, 0, info) as Uint8Array | null
      surface.delete()
      if (!px) return
      ;[c.width, c.height] = [width, height]
      c.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(px), width), 0, 0)
    }
    const run = () => {
      const end = performance.now() + 8
      for (const [c, [lists, sheets]] of queue) {
        if (performance.now() > end) break
        queue.delete(c)
        draw(c, lists, sheets)
      }
      frame = queue.size ? requestAnimationFrame(run) : 0
    }
    return {
      paint: (c: HTMLCanvasElement, lists: Lists, sheets: Sheet[]) => {
        queue.set(c, [lists, sheets])
        frame ||= requestAnimationFrame(run)
      },
      stop: () => {
        cancelAnimationFrame(frame)
        frame = 0
        queue.clear()
        renderer?.delete()
        renderer = undefined
      },
    }
  })
  useEffect(() => painter.stop, [painter])
  return painter.paint
}

/** A picture of the pages `sheets` drawn from `lists`, `height` px tall, redrawn when the document changes. */
function Thumb({ paint, snapshot, lists, sheets, height }: { paint: ReturnType<typeof usePaint>; snapshot: Snapshot; lists: Lists; sheets: Sheet[]; height: number }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const w = Math.max(...sheets.map((s) => s.x + s.width)) - Math.min(...sheets.map((s) => s.x))
  const h = Math.max(...sheets.map((s) => s.height))
  const key = JSON.stringify([lists, sheets])
  useEffect(() => {
    const [l, s] = JSON.parse(key)
    paint(ref.current!, l, s)
  }, [paint, snapshot, key])
  return <canvas ref={ref} className="thumb" style={{ width: Math.round((height * w) / h), height }} />
}

const sheet = (p: Page, x = 0): Sheet => ({ x, width: p.width, height: p.height, bleed: p.bleed })
const width = (p: Page) => Math.round((PAGE * p.width) / p.height)

/**
 * The page overview over the canvas: the masters in a row, then the spreads one under
 * another. Click selects pages, double-click or Enter opens one; they are dragged or
 * moved with Alt ←/→ to reorder, deleted with Del and have a context menu.
 */
export function Overview({ ck, editor }: { ck: CanvasKit; editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const selected = useEditor(editor, (e) => e.overview) ?? []
  const current = useEditor(editor, (e) => e.pageId)
  const paint = usePaint(ck, editor)
  const list = useRef<HTMLDivElement>(null)
  const [anchor, setAnchor] = useState(selected[0])
  const [menu, setMenu] = useState<Menu | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [dragging, setDragging] = useState(false)
  const [drop, setDrop] = useState<{ id: string; before: boolean } | null>(null)
  const { pages, masters, spreads, facingPages: facing } = snapshot
  const ids = pages.map((p) => p.id)

  useEffect(() => list.current?.focus(), [])

  const select = (sel: string[]) => editor.set({ overview: sel })
  const close = () => editor.set({ overview: null })
  const openPage = (id: string) => {
    editor.showPage(id)
    close()
  }
  const plural = (n: number) => `${n} ${n === 1 ? 'page' : 'pages'}`
  const click = (e: MouseEvent, id: string) => {
    list.current?.focus()
    if (e.detail === 2) return openPage(id)
    if (e.shiftKey) {
      const [a, b] = [ids.indexOf(anchor ?? id), ids.indexOf(id)].sort((x, y) => x - y)
      return select(ids.slice(a, b + 1))
    }
    setAnchor(id)
    if (e.ctrlKey || e.metaKey) select(selected.includes(id) ? selected.filter((s) => s !== id) : [...selected, id])
    else select([id])
  }
  /** Moves the pages `moving` so that they follow each other at `at` among the other pages. */
  const move = (moving: string[], at: number) => {
    const others = ids.filter((id) => !moving.includes(id))
    const order = [...others.slice(0, at), ...moving, ...others.slice(at)]
    editor.batch(() => {
      order.forEach((id, i) => {
        if (editor.snapshot.pages[i].id !== id) editor.apply({ type: 'movePage', id, index: i })
      })
    })
  }
  const remove = (del: string[]) => {
    if (del.length >= pages.length) return editor.say('A document keeps at least 1 page')
    const numbers = del.map((id) => ids.indexOf(id) + 1)
    const next = ids.find((id, i) => !del.includes(id) && i >= Math.min(...numbers) - 1) ?? ids.findLast((id) => !del.includes(id))!
    editor.batch(() => {
      for (const id of del) editor.apply({ type: 'deletePage', id })
    })
    select([next])
    editor.say(`Deleted ${del.length === 1 ? `page ${numbers[0]}` : plural(del.length)}. Ctrl Z restores`)
  }
  const show = (id?: string) => {
    if (!id) return
    editor.showPage(id)
    select([id])
  }
  const addPage = () => show(editor.apply({ type: 'addPage', after: ids.filter((id) => selected.includes(id)).at(-1) ?? null })[0])
  const addMaster = (like: string) => {
    const [id] = editor.apply({ type: 'addMaster', like })
    if (id) openPage(id)
  }
  const rename = (id: string, name: string | null) => {
    if (name) editor.apply({ type: 'setMaster', id, name })
    setRenaming(null)
  }
  const openMenu = (e: MouseEvent, id: string, master: boolean) => {
    e.preventDefault()
    if (!master && !selected.includes(id)) select([id])
    setMenu({ id, master, x: e.clientX, y: e.clientY })
  }

  const onKey = (e: KeyboardEvent) => {
    const last = selected.at(-1) ?? current
    const i = Math.max(0, ids.indexOf(last))
    const d = e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : 0
    if (d && e.altKey) {
      if (selected.length === 1 && ids[i + d]) move(selected, i + d)
    } else if (d) {
      const j = Math.max(0, Math.min(ids.length - 1, i + d))
      if (e.shiftKey) select([...selected.filter((id) => id !== ids[j]), ids[j]])
      else {
        setAnchor(ids[j])
        select([ids[j]])
      }
    } else if (e.key === 'Delete' || e.key === 'Backspace') remove(selected.length ? selected : [last])
    else if (e.key === 'Enter') openPage(ids[i])
    else return
    e.preventDefault()
  }

  const over = (e: DragEvent, id: string) => {
    if (!dragging) return
    e.preventDefault()
    const r = e.currentTarget.getBoundingClientRect()
    const before = e.clientX < r.left + r.width / 2
    if (drop?.id !== id || drop.before !== before) setDrop({ id, before })
  }
  const onDrop = (e: DragEvent) => {
    e.preventDefault()
    if (drop && !selected.includes(drop.id)) {
      const others = ids.filter((id) => !selected.includes(id))
      move(
        ids.filter((id) => selected.includes(id)),
        others.indexOf(drop.id) + (drop.before ? 0 : 1),
      )
    }
    setDrop(null)
  }

  const pageThumb = (p: Page) => {
    const n = ids.indexOf(p.id) + 1
    const master = masters.find((m) => m.id === p.master)
    return (
      <div
        key={p.id}
        id={`ov-${p.id}`}
        role="option"
        className="ov-page"
        aria-selected={selected.includes(p.id)}
        aria-current={p.id === current ? 'page' : undefined}
        aria-label={`Page ${n}`}
        title={`Page ${n}${master ? `, master ${master.name}` : ''}`}
        data-drop={drop?.id === p.id ? (drop.before ? 'before' : 'after') : undefined}
        draggable
        onClick={(e) => click(e, p.id)}
        onContextMenu={(e) => openMenu(e, p.id, false)}
        onDragStart={(e) => {
          if (!selected.includes(p.id)) select([p.id])
          e.dataTransfer.effectAllowed = 'move'
          e.dataTransfer.setData('text/plain', p.id)
          setDragging(true)
        }}
        onDragOver={(e) => over(e, p.id)}
        onDragLeave={() => setDrop(null)}
        onDrop={onDrop}
        onDragEnd={() => {
          setDragging(false)
          setDrop(null)
        }}
      >
        <Thumb paint={paint} snapshot={snapshot} lists={[{ id: p.id, x: 0 }]} sheets={[sheet(p)]} height={PAGE} />
      </div>
    )
  }

  return (
    <section className="overview" aria-label="Page overview" onPointerDown={editor.gesture}>
      <h2>
        Masters<span>Double-click to edit</span>
      </h2>
      <div className="ov-row" role="group" aria-label="Masters" onKeyDown={(e) => roam(e, [...e.currentTarget.querySelectorAll('.ov-master')])}>
        {masters.map((m, k) => (
          <div key={m.id} className="ov-item" data-current={m.id === current || undefined}>
            <button
              type="button"
              className="ov-master"
              aria-label={m.name}
              aria-current={m.id === current ? 'page' : undefined}
              title={`Double-click to edit ${m.name}`}
              tabIndex={k ? -1 : 0}
              onClick={(e) => e.detail === 2 && openPage(m.id)}
              onContextMenu={(e) => openMenu(e, m.id, true)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') openPage(m.id)
                else if (e.key === 'F2') setRenaming(m.id)
                else return
                e.preventDefault()
              }}
            >
              <Thumb
                paint={paint}
                snapshot={snapshot}
                lists={[{ id: m.id, x: 0 }]}
                sheets={facing ? [sheet(m, -m.width), sheet(m)] : [sheet(m)]}
                height={MASTER}
              />
            </button>
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
                  e.stopPropagation()
                }}
              />
            ) : (
              <span className="ov-cap" onDoubleClick={() => setRenaming(m.id)}>
                {m.name}
              </span>
            )}
          </div>
        ))}
        <button type="button" className="ov-add" onClick={() => addMaster(selected.at(-1) ?? current)}>
          <Icon name="plus" />
          New master
        </button>
      </div>
      <h2>
        Pages {pages.length}
        <span>Click selects, double-click opens, Alt ←/→ moves, Del deletes</span>
      </h2>
      <div
        ref={list}
        className="ov-spreads"
        role="listbox"
        aria-label="Pages"
        aria-multiselectable
        aria-activedescendant={selected.length ? `ov-${selected.at(-1)}` : undefined}
        tabIndex={0}
        onKeyDown={onKey}
      >
        {(facing ? spreads : ids.map((id) => [id])).map((spread, k) => (
          <div
            key={spread.join()}
            className="ov-item"
            role={facing ? 'group' : undefined}
            aria-label={facing ? `Spread ${spread.map((id) => ids.indexOf(id) + 1).join('–')}` : undefined}
            data-current={spread.includes(current) || undefined}
          >
            <div className="ov-pages" style={facing && k === 0 && spread.length === 1 ? { paddingLeft: width(pages[0]) } : undefined}>
              {spread.map((id) => pageThumb(pages[ids.indexOf(id)]))}
            </div>
            <span className="ov-cap">
              {spread.map((id) => {
                const m = masters.find((m) => m.id === pages[ids.indexOf(id)].master)
                return (
                  <span key={id}>
                    {ids.indexOf(id) + 1}
                    {m && <b title={`Master ${m.name}`}>{prefix(m.name)}</b>}
                  </span>
                )
              })}
            </span>
          </div>
        ))}
        <button type="button" className="ov-add" onClick={addPage}>
          <Icon name="plus" />
          Add page
        </button>
      </div>
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
                    ['Duplicate page', () => show(editor.apply({ type: 'duplicatePage', id: menu.id })[0]), true],
                    ['Delete page', () => remove(selected), selected.length < pages.length],
                    ['Master like page', () => addMaster(menu.id), true],
                  ]
            }
          />,
          document.body,
        )}
    </section>
  )
}
