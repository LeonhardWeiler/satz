import { useEffect, useRef, useState } from 'react'
import type { CanvasKit, Surface } from 'canvaskit-wasm'
import { MM, bounds, type Editor } from './editor'
import type { Node } from './model'
import { Renderer, fitView, HANDLE, type Box, type View } from './renderer'
import { pick } from './select'

const PX_PER_PT = 96 / 72
const DRAG = 3
const EDGE = 4
const CURSORS: Record<string, string> = {
  nw: 'nwse-resize',
  se: 'nwse-resize',
  ne: 'nesw-resize',
  sw: 'nesw-resize',
  n: 'ns-resize',
  s: 'ns-resize',
  e: 'ew-resize',
  w: 'ew-resize',
}
const DEFAULT_SIZE = { rect: [30, 30], frame: [30, 30], text: [60, 12] } as const

type Point = { x: number; y: number }
type Drag =
  | { kind: 'pan'; last: Point }
  | { kind: 'move'; start: Point; frames: Node[]; active: boolean }
  | { kind: 'resize'; start: Point; handle: string; box: Box; frames: Node[] }
  | { kind: 'marquee'; start: Point; end: Point; base: string[] }
  | { kind: 'draw'; start: Point; id: string; moved: boolean; tool: keyof typeof DEFAULT_SIZE }

export function isTyping(e: Event) {
  return e.target instanceof HTMLElement && e.target.closest('input, textarea, select, [contenteditable]') !== null
}

export function Canvas({ ck, editor }: { ck: CanvasKit; editor: Editor }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const [zoom, setZoom] = useState(0)

  useEffect(() => {
    const canvas = ref.current!
    const renderer = new Renderer(ck, editor.engine)
    const view: View = { x: 0, y: 0, zoom: 1 }
    let surface: Surface | null = null
    let frame = 0
    let fitted = false
    let space = false
    let drag: Drag | null = null
    let hover: string | undefined

    const toDoc = (e: { offsetX: number; offsetY: number }): Point => ({
      x: (e.offsetX - view.x) / view.zoom,
      y: (e.offsetY - view.y) / view.zoom,
    })
    const rect = (a: Point, b: Point): Box => ({
      x: Math.min(a.x, b.x),
      y: Math.min(a.y, b.y),
      w: Math.abs(a.x - b.x),
      h: Math.abs(a.y - b.y),
    })
    const selectionBox = () => {
      const nodes = editor.selected()
      return nodes.length ? bounds(nodes) : undefined
    }

    const redraw = () => {
      if (frame) return
      frame = requestAnimationFrame(() => {
        frame = 0
        if (!surface) return
        const box = editor.tool === 'move' && drag?.kind !== 'marquee' ? selectionBox() : undefined
        const hovered = hover && !editor.selection.includes(hover) ? editor.nodes.get(hover)?.node : undefined
        const marquee =
          drag?.kind === 'marquee'
            ? rect(
                { x: view.x + drag.start.x * view.zoom, y: view.y + drag.start.y * view.zoom },
                { x: view.x + drag.end.x * view.zoom, y: view.y + drag.end.y * view.zoom },
              )
            : undefined
        renderer.draw(surface.getCanvas(), view, canvas.width / canvas.clientWidth, {
          selection: editor.selection.length > 1 ? editor.selected() : [],
          hover: hovered,
          marquee,
          handles: box,
        })
        surface.flush()
      })
    }
    const zoomAt = (px: number, py: number, zoom: number) => {
      zoom = Math.min(256, Math.max(0.02, zoom))
      view.x = px - ((px - view.x) * zoom) / view.zoom
      view.y = py - ((py - view.y) * zoom) / view.zoom
      view.zoom = zoom
      setZoom(zoom)
      redraw()
    }
    const fit = () => {
      Object.assign(view, fitView(editor.page, canvas.clientWidth, canvas.clientHeight))
      setZoom(view.zoom)
      redraw()
    }

    const handleAt = (e: PointerEvent) => {
      const box = editor.tool === 'move' ? selectionBox() : undefined
      if (!box) return undefined
      const l = view.x + box.x * view.zoom
      const t = view.y + box.y * view.zoom
      const r = l + box.w * view.zoom
      const b = t + box.h * view.zoom
      const near = (v: number, a: number, d: number) => Math.abs(v - a) <= d
      const within = (v: number, a: number, c: number) => v >= a - EDGE && v <= c + EDGE
      const { offsetX: x, offsetY: y } = e
      const v = near(y, t, HANDLE / 2) ? 'n' : near(y, b, HANDLE / 2) ? 's' : ''
      const h = near(x, l, HANDLE / 2) ? 'w' : near(x, r, HANDLE / 2) ? 'e' : ''
      if (v && h) return v + h
      if (!within(x, l, r) || !within(y, t, b)) return undefined
      if (near(y, t, EDGE)) return 'n'
      if (near(y, b, EDGE)) return 's'
      if (near(x, l, EDGE)) return 'w'
      if (near(x, r, EDGE)) return 'e'
      return undefined
    }
    const hit = (p: Point) => editor.engine.hit(0, p.x, p.y)

    const resize = new ResizeObserver(([entry]) => {
      const box = entry.devicePixelContentBoxSize?.[0]
      canvas.width = box ? box.inlineSize : Math.round(entry.contentRect.width * devicePixelRatio)
      canvas.height = box ? box.blockSize : Math.round(entry.contentRect.height * devicePixelRatio)
      surface?.delete()
      surface = ck.MakeWebGLCanvasSurface(canvas)
      if (!fitted) {
        fitted = true
        fit()
      }
      redraw()
    })
    try {
      resize.observe(canvas, { box: 'device-pixel-content-box' })
    } catch {
      resize.observe(canvas)
    }

    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      const scale = e.deltaMode === 1 ? 16 : 1
      if (e.ctrlKey || e.metaKey) {
        zoomAt(e.offsetX, e.offsetY, view.zoom * Math.exp(-e.deltaY * scale * 0.01))
      } else {
        view.x -= (e.shiftKey && !e.deltaX ? e.deltaY : e.deltaX) * scale
        view.y -= (e.shiftKey && !e.deltaX ? 0 : e.deltaY) * scale
        redraw()
      }
    }
    const onPointerDown = (e: PointerEvent) => {
      if (e.button !== 0 && e.button !== 1) return
      canvas.focus()
      canvas.setPointerCapture(e.pointerId)
      const p = toDoc(e)
      if (e.button === 1 || space) {
        e.preventDefault()
        drag = { kind: 'pan', last: { x: e.clientX, y: e.clientY } }
        canvas.dataset.panning = ''
        return
      }
      if (editor.tool !== 'move') {
        const path = hit(p)
        const parent = path.findLast((id) => editor.nodes.get(id)?.node.kind === 'frame') ?? editor.page.id
        const tool = editor.tool
        editor.apply({ type: 'beginUndoGroup' })
        const [id] = editor.apply({ type: 'create', parent, kind: tool, x: p.x, y: p.y, w: 0, h: 0 })
        drag = { kind: 'draw', start: p, id, moved: false, tool }
        editor.set({ selection: [id] })
        return
      }
      const handle = handleAt(e)
      if (handle) {
        const frames = editor.selected()
        drag = { kind: 'resize', start: p, handle, box: bounds(frames), frames }
        editor.apply({ type: 'beginUndoGroup' })
        return
      }
      const mode = e.ctrlKey || e.metaKey ? 'deep' : 'click'
      const id = pick(editor.page.children, hit(p), editor.selection, mode)
      const sel = editor.selection
      if (!id) {
        drag = { kind: 'marquee', start: p, end: p, base: e.shiftKey ? sel : [] }
        editor.set({ selection: drag.base })
        return
      }
      if (e.shiftKey) {
        editor.set({ selection: sel.includes(id) ? sel.filter((s) => s !== id) : [...sel, id] })
        return
      }
      if (!sel.includes(id)) editor.set({ selection: [id] })
      drag = { kind: 'move', start: p, frames: editor.selected(), active: false }
    }
    const onPointerMove = (e: PointerEvent) => {
      const p = toDoc(e)
      if (!drag) {
        canvas.style.cursor = CURSORS[handleAt(e) ?? ''] ?? ''
        const id = editor.tool === 'move' ? pick(editor.page.children, hit(p), editor.selection, 'click') : undefined
        if (id !== hover) {
          hover = id
          redraw()
        }
        return
      }
      if (drag.kind === 'pan') {
        view.x += e.clientX - drag.last.x
        view.y += e.clientY - drag.last.y
        drag.last = { x: e.clientX, y: e.clientY }
        redraw()
      } else if (drag.kind === 'marquee') {
        drag.end = p
        const m = rect(drag.start, p)
        const inside = editor.page.children
          .filter((n) => n.x < m.x + m.w && n.x + n.w > m.x && n.y < m.y + m.h && n.y + n.h > m.y)
          .map((n) => n.id)
        editor.set({ selection: [...new Set([...drag.base, ...inside])] })
      } else if (drag.kind === 'draw') {
        drag.moved ||= Math.hypot(p.x - drag.start.x, p.y - drag.start.y) * view.zoom > DRAG
        if (!drag.moved) return
        let dx = p.x - drag.start.x
        let dy = p.y - drag.start.y
        if (e.shiftKey) {
          const d = Math.max(Math.abs(dx), Math.abs(dy))
          dx = Math.sign(dx || 1) * d
          dy = Math.sign(dy || 1) * d
        }
        const a = e.altKey ? { x: drag.start.x - dx, y: drag.start.y - dy } : drag.start
        const r = rect(a, { x: drag.start.x + dx, y: drag.start.y + dy })
        editor.apply({ type: 'setFrame', id: drag.id, ...r })
      } else if (drag.kind === 'move') {
        let dx = p.x - drag.start.x
        let dy = p.y - drag.start.y
        if (!drag.active) {
          if (Math.hypot(dx, dy) * view.zoom <= DRAG) return
          drag.active = true
          editor.apply({ type: 'beginUndoGroup' })
        }
        if (e.shiftKey) {
          if (Math.abs(dx) > Math.abs(dy)) dy = 0
          else dx = 0
        }
        for (const n of drag.frames) editor.apply({ type: 'setFrame', id: n.id, x: n.x + dx, y: n.y + dy, w: n.w, h: n.h })
      } else if (drag.kind === 'resize') {
        const { box, handle } = drag
        const dx = p.x - drag.start.x
        const dy = p.y - drag.start.y
        let l = box.x + (handle.includes('w') ? dx : 0)
        let r = box.x + box.w + (handle.includes('e') ? dx : 0)
        let t = box.y + (handle.includes('n') ? dy : 0)
        let b = box.y + box.h + (handle.includes('s') ? dy : 0)
        if (e.altKey) {
          if (handle.includes('w')) r = box.x + box.w - dx
          if (handle.includes('e')) l = box.x - dx
          if (handle.includes('n')) b = box.y + box.h - dy
          if (handle.includes('s')) t = box.y - dy
        }
        if (e.shiftKey && box.w && box.h) {
          const s = Math.max(Math.abs((r - l) / box.w), Math.abs((b - t) / box.h))
          const cx = handle.includes('w') ? r : handle.includes('e') ? l : (l + r) / 2
          const cy = handle.includes('n') ? b : handle.includes('s') ? t : (t + b) / 2
          const w = box.w * s * Math.sign(r - l || 1)
          const h = box.h * s * Math.sign(b - t || 1)
          ;[l, r] = handle.includes('w') ? [cx - w, cx] : handle.includes('e') ? [cx, cx + w] : [cx - w / 2, cx + w / 2]
          ;[t, b] = handle.includes('n') ? [cy - h, cy] : handle.includes('s') ? [cy, cy + h] : [cy - h / 2, cy + h / 2]
        }
        const sx = box.w ? (r - l) / box.w : 1
        const sy = box.h ? (b - t) / box.h : 1
        for (const n of drag.frames) {
          const x1 = l + (n.x - box.x) * sx
          const y1 = t + (n.y - box.y) * sy
          const f = rect({ x: x1, y: y1 }, { x: x1 + n.w * sx, y: y1 + n.h * sy })
          editor.apply({ type: 'setFrame', id: n.id, ...f })
        }
      }
    }
    const onPointerUp = (e: PointerEvent) => {
      if (drag?.kind === 'draw') {
        if (!drag.moved) {
          const [w, h] = DEFAULT_SIZE[drag.tool]
          editor.apply({ type: 'setFrame', id: drag.id, x: drag.start.x, y: drag.start.y, w: w * MM, h: h * MM })
        }
        editor.set({ tool: 'move' })
      }
      if (drag?.kind === 'draw' || drag?.kind === 'resize' || (drag?.kind === 'move' && drag.active)) {
        editor.apply({ type: 'endUndoGroup' })
      }
      if (drag?.kind === 'move' && !drag.active && !e.shiftKey) {
        const id = pick(editor.page.children, hit(toDoc(e)), editor.selection, e.ctrlKey || e.metaKey ? 'deep' : 'click')
        if (id && editor.selection.length > 1) editor.set({ selection: [id] })
      }
      drag = null
      delete canvas.dataset.panning
      redraw()
    }
    const onDoubleClick = (e: MouseEvent) => {
      if (editor.tool !== 'move') return
      const id = pick(editor.page.children, hit(toDoc(e)), editor.selection, 'double')
      if (id) editor.set({ selection: [id] })
    }
    const onLeave = () => {
      hover = undefined
      redraw()
    }
    const onKey = (e: KeyboardEvent) => {
      if (isTyping(e)) return
      if (e.code === 'Space') {
        space = e.type === 'keydown'
        canvas.toggleAttribute('data-space', space)
        e.preventDefault()
        return
      }
      if (e.type !== 'keydown') return
      const cx = canvas.clientWidth / 2
      const cy = canvas.clientHeight / 2
      const mod = e.ctrlKey || e.metaKey
      if (e.shiftKey && e.code === 'Digit1') fit()
      else if (e.shiftKey && e.code === 'Digit0') zoomAt(cx, cy, PX_PER_PT)
      else if (mod && (e.key === '=' || e.key === '+')) zoomAt(cx, cy, view.zoom * 2)
      else if (mod && e.key === '-') zoomAt(cx, cy, view.zoom / 2)
      else return
      e.preventDefault()
    }

    const unsubscribe = editor.subscribe(() => {
      canvas.dataset.tool = editor.tool
      redraw()
    })
    canvas.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('pointercancel', onPointerUp)
    canvas.addEventListener('pointerleave', onLeave)
    canvas.addEventListener('dblclick', onDoubleClick)
    window.addEventListener('keydown', onKey)
    window.addEventListener('keyup', onKey)
    return () => {
      cancelAnimationFrame(frame)
      unsubscribe()
      resize.disconnect()
      canvas.removeEventListener('wheel', onWheel)
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.removeEventListener('pointercancel', onPointerUp)
      canvas.removeEventListener('pointerleave', onLeave)
      canvas.removeEventListener('dblclick', onDoubleClick)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('keyup', onKey)
      renderer.delete()
      surface?.delete()
    }
  }, [ck, editor])

  return (
    <div className="stage">
      <canvas ref={ref} className="canvas" aria-label="Page canvas" tabIndex={-1} data-tool="move" />
      <output className="zoom" aria-label="Zoom">
        {Math.round((zoom / PX_PER_PT) * 100)}%
      </output>
    </div>
  )
}
