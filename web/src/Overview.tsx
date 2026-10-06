import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactNode } from 'react'
import type { CanvasKit, GrDirectContext } from 'canvaskit-wasm'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { Field } from './controls'
import { prefix } from './engine/engine'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'
import type { Page, Snapshot } from './model'
import { Renderer, type Sheet } from './renderer'
import { setSettings, settings, useSettings } from './settings'

type Lists = { id: string; x: number }[]
type Menu = { id: string; master: boolean; x: number; y: number }

/** The smallest and largest page height in the overview in px. */
const ZOOM = [80, 480]
const clamp = (v: number, [lo, hi]: number[]) => Math.round(Math.min(hi, Math.max(lo, v)))
const resize = (patch: Partial<typeof settings.overview>) => setSettings({ overview: { ...settings.overview, ...patch } })

let painter: ReturnType<typeof makePainter> | undefined

/**
 * Draws pictures of pages with the canvas renderer on the GPU, as many per frame as fit in 8 ms. It lives on
 * after the overview closes and keeps the last picture of each page, which a canvas shows until it is drawn again.
 */
function makePainter(ck: CanvasKit, editor: Editor) {
  let renderer: Renderer | undefined
  let gpu: GrDirectContext | null = null
  const off = new OffscreenCanvas(1, 1)
  const queue = new Map<HTMLCanvasElement, [Lists, Sheet[]]>()
  const drawn = new Map<HTMLCanvasElement, [Lists, Sheet[]]>()
  const pictures = new Map<string, ImageBitmap>()
  const show = (c: HTMLCanvasElement, picture?: ImageBitmap) => {
    if (!picture) return
    ;[c.width, c.height] = [Math.round(c.clientWidth * devicePixelRatio), Math.round(c.clientHeight * devicePixelRatio)]
    c.getContext('2d')!.drawImage(picture, 0, 0, c.width, c.height)
  }
  let frame = 0
  const draw = (c: HTMLCanvasElement, lists: Lists, sheets: Sheet[]) => {
    if (!c.isConnected) return
    drawn.set(c, [lists, sheets])
    const left = Math.min(...sheets.map((s) => s.x))
    const width = Math.round(c.clientWidth * devicePixelRatio)
    const height = Math.round(c.clientHeight * devicePixelRatio)
    const zoom = width / (Math.max(...sheets.map((s) => s.x + s.width)) - left)
    ;[off.width, off.height] = [width, height]
    gpu ??= ck.MakeWebGLContext(ck.GetWebGLContext(off))
    const surface = gpu && ck.MakeOnScreenGLSurface(gpu, width, height, ck.ColorSpace.SRGB)
    if (!surface) return
    renderer ??= new Renderer(ck, editor.engine, () => {
      for (const [c, args] of drawn) if (c.isConnected) queue.set(c, args)
      frame ||= requestAnimationFrame(run)
    }, 1024)
    renderer.reprofile(editor.snapshot.profile ?? '')
    renderer.draw(surface.getCanvas(), lists, sheets, { x: -left * zoom, y: 0, zoom }, 1, null, editor.snapshot.colorMode === 'cmyk')
    surface.flush()
    surface.delete()
    const key = JSON.stringify([lists, sheets])
    pictures.get(key)?.close()
    pictures.set(key, off.transferToImageBitmap())
    show(c, pictures.get(key))
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
      show(c, pictures.get(JSON.stringify([lists, sheets])))
      queue.set(c, [lists, sheets])
      frame ||= requestAnimationFrame(run)
    },
    stop: () => {
      cancelAnimationFrame(frame)
      frame = 0
      queue.clear()
      drawn.clear()
      renderer?.sweep()
    },
  }
}

function usePaint(ck: CanvasKit, editor: Editor) {
  const p = (painter ??= makePainter(ck, editor))
  useEffect(() => p.stop, [p])
  return p.paint
}

/** A picture of the pages `sheets` drawn from `lists`, `height` px tall, redrawn when the document changes. */
function Thumb({ paint, snapshot, lists, sheets, height }: { paint: ReturnType<typeof usePaint>; snapshot: Snapshot; lists: Lists; sheets: Sheet[]; height: number }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const w = Math.max(...sheets.map((s) => s.x + s.width)) - Math.min(...sheets.map((s) => s.x))
  const h = Math.max(...sheets.map((s) => s.height))
  const key = JSON.stringify([lists, sheets, height])
  useEffect(() => {
    const [l, s] = JSON.parse(key)
    paint(ref.current!, l, s)
  }, [paint, snapshot, key])
  return <canvas ref={ref} className="thumb" style={{ width: Math.round((height * w) / h), height }} />
}

/** Shows the thumbnails of `ids` stacked behind `grabbed` as the drag image, with their count. */
function ghost(data: DataTransfer, grabbed: HTMLElement, ids: string[]) {
  const deck = document.createElement('div')
  deck.className = 'ov-ghost'
  const cards = [...ids.filter((id) => `ov-${id}` !== grabbed.id).slice(0, 4).reverse().map((id) => document.getElementById(`ov-${id}`)), grabbed]
  const h = Math.min(grabbed.clientHeight, 160)
  cards.forEach((card, i) => {
    const src = card?.querySelector('canvas')
    if (!src) return
    const c = document.createElement('canvas')
    ;[c.width, c.height] = [src.width, src.height]
    c.getContext('2d')!.drawImage(src, 0, 0)
    const at = (cards.length - 1 - i) * 8
    Object.assign(c.style, { left: `${at}px`, top: `${at}px`, width: `${(h * src.width) / src.height}px`, height: `${h}px` })
    deck.append(c)
  })
  const count = document.createElement('b')
  count.textContent = String(ids.length)
  deck.append(count)
  document.body.append(deck)
  data.setDragImage(deck, 16, 16)
  setTimeout(() => deck.remove())
}

const sheet = (p: Page, x = 0): Sheet => ({ x, width: p.width, height: p.height, bleed: p.bleed })

/**
 * The page overview over the canvas: the masters in a column, the spreads beside them one
 * under another. A click or Enter opens a page or master; Ctrl and Shift click and a band dragged
 * over the empty space select them in their list; the selected ones are dragged or moved with Alt and an arrow key to reorder,
 * deleted with Del and have a context menu.
 */
export function Overview({ ck, editor }: { ck: CanvasKit; editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const selected = useEditor(editor, (e) => e.overview) ?? []
  const current = useEditor(editor, (e) => e.pageId)
  const paint = usePaint(ck, editor)
  const list = useRef<HTMLDivElement>(null)
  const masterList = useRef<HTMLDivElement>(null)
  const [anchor, setAnchor] = useState(selected[0])
  const [menu, setMenu] = useState<Menu | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [dragging, setDragging] = useState<string[] | null>(null)
  const [drop, setDrop] = useState<{ id: string; before: boolean } | null>(null)
  const [band, setBand] = useState<DOMRect | null>(null)
  const { pages, masters, spreads, facingPages: facing } = snapshot
  const ids = pages.map((p) => p.id)
  const masterIds = masters.map((m) => m.id)
  const listOf = (id: string) => (masterIds.includes(id) ? masterIds : ids)
  const { masters: wide, pages: PAGE, columns } = useSettings().overview
  const width = (p: Page) => Math.round((PAGE * p.width) / p.height)
  const grab = useRef(0)
  const scroller = useRef<HTMLDivElement>(null)

  useEffect(() => list.current?.focus(), [])
  useEffect(() => {
    const el = scroller.current!
    const wheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return
      e.preventDefault()
      resize({ pages: clamp(settings.overview.pages * Math.exp(-e.deltaY / 300), ZOOM) })
    }
    el.addEventListener('wheel', wheel, { passive: false })
    return () => el.removeEventListener('wheel', wheel)
  }, [])

  const select = (sel: string[]) => editor.set({ overview: sel })
  const close = () => editor.set({ overview: null })
  const openPage = (id: string) => {
    editor.showPage(id)
    close()
  }
  const click = (e: MouseEvent, id: string) => {
    const all = listOf(id)
    if (!e.ctrlKey && !e.metaKey && !e.shiftKey) return openPage(id)
    ;(all === ids ? list : masterList).current?.focus()
    if (e.shiftKey && anchor && all.includes(anchor)) {
      const [a, b] = [all.indexOf(anchor), all.indexOf(id)].sort((x, y) => x - y)
      return select(all.slice(a, b + 1))
    }
    setAnchor(id)
    const mine = selected.filter((s) => all.includes(s))
    select(mine.includes(id) ? mine.filter((s) => s !== id) : [...mine, id])
  }
  /** Selects the pages or masters of `all` that a band dragged from the empty space of a list touches, with Ctrl or Shift added to those selected. */
  const marquee = (e: PointerEvent<HTMLElement>, all: string[], box: HTMLElement | null) => {
    if (e.button !== 0 || (e.target as Element).closest('.ov-item, button, input, .ov-head')) return
    e.preventDefault()
    box?.focus()
    const el = e.currentTarget
    el.setPointerCapture(e.pointerId)
    const [x, y] = [e.clientX, e.clientY]
    const base = e.ctrlKey || e.metaKey || e.shiftKey ? selected.filter((id) => all.includes(id)) : []
    select(base)
    const moved = (m: globalThis.PointerEvent) => {
      const r = new DOMRect(Math.min(x, m.clientX), Math.min(y, m.clientY), Math.abs(m.clientX - x), Math.abs(m.clientY - y))
      setBand(r)
      const hit = all.filter((id) => {
        const o = document.getElementById(`ov-${id}`)!.getBoundingClientRect()
        return o.left < r.right && r.left < o.right && o.top < r.bottom && r.top < o.bottom
      })
      select([...base.filter((id) => !hit.includes(id)), ...hit])
    }
    const up = () => {
      el.removeEventListener('pointermove', moved)
      el.removeEventListener('pointerup', up)
      setBand(null)
    }
    el.addEventListener('pointermove', moved)
    el.addEventListener('pointerup', up)
  }
  const move = (moving: string[], index: number) => editor.apply({ type: 'movePages', ids: moving, index })
  const show = (id?: string) => {
    if (!id) return
    editor.showPage(id)
    select([id])
  }
  const addPage = () => show(editor.apply({ type: 'addPage', after: ids.at(-1) ?? null })[0])
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
    if (!selected.includes(id)) select([id])
    setMenu({ id, master, x: e.clientX, y: e.clientY })
  }

  const onKey = (e: KeyboardEvent, all: string[]) => {
    const mine = selected.filter((id) => all.includes(id))
    const i = Math.max(0, all.indexOf(mine.at(-1) ?? current))
    const d = e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : 0
    if (d && e.altKey) {
      if (mine.length === 1 && all[i + d]) move(mine, i + d)
    } else if (d) {
      const j = Math.max(0, Math.min(all.length - 1, i + d))
      if (e.shiftKey) select([...mine.filter((id) => id !== all[j]), all[j]])
      else {
        setAnchor(all[j])
        select([all[j]])
      }
    } else if (e.key === 'Delete' || e.key === 'Backspace') {
      if (all === ids) editor.deletePages(mine.length ? mine : [all[i]])
      else if (mine.length) editor.batch(() => mine.forEach((id) => editor.apply({ type: 'deleteMaster', id })))
    } else if (e.key === 'Enter' && all[i]) openPage(all[i])
    else if (e.key === 'F2' && all === masterIds && all[i]) setRenaming(all[i])
    else return
    e.preventDefault()
  }

  /** A page or master `id` in the overview, `vertical` when its list runs down. */
  const thumb = (id: string, label: string, title: string, vertical: boolean, picture: ReactNode) => (
    <div
      key={id}
      id={`ov-${id}`}
      role="option"
      className={vertical ? 'ov-master' : 'ov-page'}
      aria-selected={selected.includes(id)}
      aria-current={id === current ? 'page' : undefined}
      aria-label={label}
      title={title}
      data-drop={drop?.id === id ? (drop.before ? 'before' : 'after') : undefined}
      draggable
      onClick={(e) => click(e, id)}
      onContextMenu={(e) => openMenu(e, id, vertical)}
      onDragStart={(e) => {
        if (!selected.includes(id)) {
          setAnchor(id)
          select([id])
        } else if (selected.length > 1) ghost(e.dataTransfer, e.currentTarget, selected)
        e.dataTransfer.effectAllowed = 'move'
        e.dataTransfer.setData('text/plain', id)
        setDragging(listOf(id))
      }}
      onDragOver={(e) => {
        if (!dragging?.includes(id)) return
        e.preventDefault()
        const r = e.currentTarget.getBoundingClientRect()
        const before = vertical ? e.clientY < r.top + r.height / 2 : e.clientX < r.left + r.width / 2
        if (drop?.id !== id || drop.before !== before) setDrop({ id, before })
      }}
      onDragLeave={() => setDrop(null)}
      onDrop={(e) => {
        e.preventDefault()
        if (dragging && drop && !selected.includes(drop.id)) {
          const others = dragging.filter((o) => !selected.includes(o))
          move(
            dragging.filter((o) => selected.includes(o)),
            others.indexOf(drop.id) + (drop.before ? 0 : 1),
          )
        }
        setDrop(null)
      }}
      onDragEnd={() => {
        setDragging(null)
        setDrop(null)
      }}
    >
      {picture}
    </div>
  )

  const pageThumb = (p: Page) => {
    const n = ids.indexOf(p.id) + 1
    const master = masters.find((m) => m.id === p.master)
    return thumb(
      p.id,
      `Page ${n}`,
      `Page ${n}${master ? `, master ${master.name}` : ''}`,
      false,
      <Thumb paint={paint} snapshot={snapshot} lists={[{ id: p.id, x: 0 }]} sheets={[sheet(p)]} height={PAGE} />,
    )
  }

  const rows = facing ? spreads : ids.map((id) => [id])
  const joins = facing && rows.length > 1 && rows.at(-1)!.length === 1
  const add = (
    <button type="button" className="ov-add ov-add-page" aria-label="Add page" title="Add page" style={{ width: width(pages.at(-1)!), height: PAGE }} onClick={addPage}>
      <Icon name="plus" />
    </button>
  )

  return (
    <section className="overview" aria-label="Page overview" style={{ gridTemplateColumns: `${wide}px 1fr` }} onPointerDown={editor.gesture}>
      <div className="ov-col" onPointerDown={(e) => marquee(e, masterIds, masterList.current)}>
        <h2>Masters</h2>
        <div
          ref={masterList}
          className="ov-row"
          role="listbox"
          aria-label="Masters"
          aria-multiselectable
          aria-activedescendant={selected.some((id) => masterIds.includes(id)) ? `ov-${selected.at(-1)}` : undefined}
          tabIndex={0}
          onKeyDown={(e) => e.target === e.currentTarget && onKey(e, masterIds)}
        >
          {masters.map((m) => (
            <div key={m.id} className="ov-item" data-current={m.id === current || undefined}>
              {thumb(
                m.id,
                m.name,
                `${m.name}, click to edit`,
                true,
                <Thumb
                  paint={paint}
                  snapshot={snapshot}
                  lists={[{ id: m.id, x: 0 }]}
                  sheets={facing ? [sheet(m, -m.width), sheet(m)] : [sheet(m)]}
                  height={Math.max(40, Math.round(((wide - 64) * m.height) / (m.width * (facing ? 2 : 1))))}
                />,
              )}
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
      </div>
      <div ref={scroller} className="ov-main" onPointerDown={(e) => marquee(e, ids, list.current)}>
        <header className="ov-head">
          <h2>Pages</h2>
          <Field label="Columns" unit="" int min={0} max={12} zero="Auto" value={columns} onCommit={(v) => resize({ columns: v })} />
        </header>
        <div className="ov-col">
          <div
            ref={list}
            style={columns ? { display: 'grid', gridTemplateColumns: `repeat(${columns}, max-content)` } : undefined}
            className="ov-spreads"
            role="listbox"
            aria-label="Pages"
            aria-multiselectable
            aria-activedescendant={selected.some((id) => ids.includes(id)) ? `ov-${selected.at(-1)}` : undefined}
            tabIndex={0}
            onKeyDown={(e) => onKey(e, ids)}
          >
            {rows.map((spread, k) => (
              <div
                key={spread.join()}
                className="ov-item"
                role={facing ? 'group' : undefined}
                aria-label={facing ? `Spread ${spread.map((id) => ids.indexOf(id) + 1).join('–')}` : undefined}
                data-current={spread.includes(current) || undefined}
              >
                <div className="ov-pages" style={facing && k === 0 && spread.length === 1 ? { paddingLeft: width(pages[0]) } : undefined}>
                  {spread.map((id) => pageThumb(pages[ids.indexOf(id)]))}
                  {k === rows.length - 1 && joins && add}
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
            {!joins && (
              <div className="ov-item">
                <div className="ov-pages">{add}</div>
              </div>
            )}
          </div>
        </div>
      </div>
      <div
        className="ov-split"
        role="separator"
        aria-orientation="vertical"
        aria-label="Masters width"
        aria-valuenow={wide}
        style={{ left: wide - 3 }}
        onPointerDown={(e) => {
          grab.current = e.clientX - wide
          e.currentTarget.setPointerCapture(e.pointerId)
          e.preventDefault()
        }}
        onPointerMove={(e) => {
          if (e.currentTarget.hasPointerCapture(e.pointerId)) resize({ masters: clamp(e.clientX - grab.current, [120, 600]) })
        }}
        onDoubleClick={() => resize({ masters: 170 })}
      />
      {band && <div className="ov-band" style={{ left: band.x, top: band.y, width: band.width, height: band.height }} />}
      {menu &&
        createPortal(
          <ContextMenu
            anchor={() => new DOMRect(menu.x, menu.y)}
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
                    ['Delete page', () => editor.deletePages(selected), selected.length < pages.length],
                    ['Master like page', () => addMaster(menu.id), true],
                  ]
            }
          />,
          document.body,
        )}
    </section>
  )
}
