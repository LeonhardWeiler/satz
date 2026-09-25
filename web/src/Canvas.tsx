import { useEffect, useRef, useState } from 'react'
import type { CanvasKit, Surface } from 'canvaskit-wasm'
import { MM, bounds, ends, insertion, useEditor, type Editor, type Point, type Tool } from './editor'
import type { Container, Node, TextNode } from './model'
import { penPath } from './pen'
import { Renderer, fitView, HANDLE, type Box, type View } from './renderer'
import { pick } from './select'
import { handleTextKey, insert, range, select, textOf, wordAt } from './textEdit'

const PX_PER_PT = 96 / 72
const DRAG = 3
const EDGE = 4
const HIT = 4
/** Distance in px of a text frame's in- and out-port from its top and bottom corner. */
const PORT_INSET = 16
const PORT = 10
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
/** Size in mm of a layer made with a click; a clicked text is auto width, a dragged one fixed. */
const DEFAULT_SIZE: Record<Exclude<Tool, 'move' | 'pen'>, [number, number]> = {
  rect: [30, 30], ellipse: [30, 30], polygon: [30, 30], star: [30, 30], frame: [30, 30], text: [0, 0],
  line: [30, 0], arrow: [30, 0],
}

type Pointer = { offsetX: number; offsetY: number; ctrlKey: boolean }
type Drag =
  | { kind: 'pan'; last: Point }
  | { kind: 'move'; start: Point; frames: Node[]; active: boolean; flow?: Container; to?: ReturnType<typeof insertion> }
  | { kind: 'resize'; start: Point; handle: string; box: Box; frames: Node[] }
  | { kind: 'end'; start: Point; id: string; ends: [Point, Point]; index: number }
  | { kind: 'marquee'; start: Point; end: Point; base: string[] }
  | { kind: 'draw'; start: Point; id: string; moved: boolean; tool: keyof typeof DEFAULT_SIZE; thread?: string }
  | { kind: 'pen'; start: Point }
  | { kind: 'text' }

/** Snaps a vector to the nearest multiple of 45°. */
function snap45(dx: number, dy: number) {
  const a = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4)
  const d = Math.hypot(dx, dy)
  return [Math.cos(a) * d, Math.sin(a) * d]
}

export function isTyping(e: Event) {
  return e.target instanceof HTMLElement && e.target.closest('input:not([type=checkbox]), textarea, select, [contenteditable]') !== null
}

/** Half a blink period of the caret in ms. */
const BLINK = 530

export function Canvas({ ck, editor }: { ck: CanvasKit; editor: Editor }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const area = useRef<HTMLTextAreaElement>(null)
  const [zoom, setZoom] = useState(0)
  const [composing, setComposing] = useState(false)
  const editing = useEditor(editor, (e) => e.editing !== null)

  useEffect(() => {
    const canvas = ref.current!
    const renderer = new Renderer(ck, editor.engine)
    const view: View = { x: 0, y: 0, zoom: 1 }
    /** The view of each page left for another, as Figma keeps it. */
    const views = new Map<string, View>()
    let shown = editor.page.id
    let surface: Surface | null = null
    let frame = 0
    let fitted = false
    let space = false
    let drag: Drag | null = null
    let hover: string | undefined
    let cursor: Point | undefined
    /** Last pointer position and Ctrl state over the canvas, for hover and cursor. */
    let pointer: Pointer | undefined
    let caretOn = true
    let blink: ReturnType<typeof setInterval> | undefined
    /** Shows the caret and blinks it from now on, so that it stays while typing. */
    const wake = () => {
      caretOn = true
      clearInterval(blink)
      blink = setInterval(() => {
        caretOn = !caretOn
        if (editor.editing) redraw()
      }, BLINK)
    }
    wake()

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
    /**
     * The in- and out-port in screen space of a single selected text frame that can
     * thread, as in InDesign: on its left edge below the top and its right edge above
     * the bottom.
     */
    const ports = () => {
      const [n] = editor.selected()
      if (editor.tool !== 'move' || editor.selection.length !== 1 || editor.editing || n?.kind !== 'text') return undefined
      if (n.sizing.horizontal === 'hug' && n.sizing.vertical === 'hug' && !n.prev && !n.next) return undefined
      return { node: n, ...portsOf(n) }
    }
    const portsOf = (n: Node) => ({
      in: { x: view.x + n.x * view.zoom, y: view.y + n.y * view.zoom + Math.min(PORT_INSET, (n.h * view.zoom) / 2) },
      out: { x: view.x + (n.x + n.w) * view.zoom, y: view.y + (n.y + n.h) * view.zoom - Math.min(PORT_INSET, (n.h * view.zoom) / 2) },
    })
    const portAt = (e: { offsetX: number; offsetY: number }) => {
      const p = ports()
      const near = (q: Point) => Math.abs(q.x - e.offsetX) <= PORT / 2 + 1 && Math.abs(q.y - e.offsetY) <= PORT / 2 + 1
      return !p ? undefined : near(p.out) ? { side: 'out' as const, node: p.node } : near(p.in) ? { side: 'in' as const, node: p.node } : undefined
    }
    /** Box handles of the selection, or the ends of a single selected line. */
    const handles = () => {
      const nodes = editor.selected()
      if (editor.tool !== 'move' || !nodes.length || editor.editing) return {}
      const line = nodes.length === 1 ? ends(nodes[0]) : undefined
      return line ? { line } : { box: bounds(nodes) }
    }

    /** Ports of the selected text frame, and lines from each frame of its thread on this page to the next. */
    const threadOverlay = () => {
      const p = drag ? undefined : ports()
      if (!p) return {}
      const n = p.node
      const state = (threaded: boolean, overset = false) => (overset ? 'overset' : threaded ? 'threaded' : 'empty') as 'overset' | 'threaded' | 'empty'
      const frames = [...editor.nodes.values()].flatMap(({ node }) => (node.kind === 'text' && node.story === n.story ? [node] : []))
      const lines = frames.flatMap((f) => {
        const next = f.next && frames.find((g) => g.id === f.next)
        return next ? [[portsOf(f).out, portsOf(next).in] as [Point, Point]] : []
      })
      return {
        ports: [
          { ...p.in, state: state(!!n.prev) },
          { ...p.out, state: state(!!n.next, n.overset) },
        ],
        threads: lines,
      }
    }

    const redraw = () => {
      if (frame) return
      frame = requestAnimationFrame(() => {
        frame = 0
        if (!surface) return
        const { box, line } = drag?.kind === 'marquee' ? {} : handles()
        const hovered = hover && !editor.selection.includes(hover) ? editor.nodes.get(hover)?.node : undefined
        const marquee =
          drag?.kind === 'marquee'
            ? rect(
                { x: view.x + drag.start.x * view.zoom, y: view.y + drag.start.y * view.zoom },
                { x: view.x + drag.end.x * view.zoom, y: view.y + drag.end.y * view.zoom },
              )
            : undefined
        const ed = editor.editing
        let text: Uint32Array | undefined
        if (ed) {
          const [x, top, bottom] = editor.engine.caret(ed.id, ed.focus)
          Object.assign(area.current?.style ?? {}, {
            left: `${view.x + x * view.zoom}px`,
            top: `${view.y + top * view.zoom}px`,
            height: `${(bottom - top) * view.zoom}px`,
          })
          if (caretOn || ed.anchor !== ed.focus) {
            text = editor.engine.textOverlay(ed.id, ed.anchor, ed.focus, 1 / view.zoom, editor.page.id).slice()
          }
        }
        renderer.draw(surface.getCanvas(), editor.page.id, view, canvas.width / canvas.clientWidth, {
          text,
          selection: editor.selection.length > 1 || ed || drag?.kind === 'draw' ? editor.selected() : [],
          hover: hovered,
          marquee,
          handles: box,
          ends: line,
          pen: editor.pen && { anchors: editor.pen.anchors, cursor: drag ? undefined : cursor },
          insert: drag?.kind === 'move' ? drag.to?.line : undefined,
          ...threadOverlay(),
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

    const handleAt = (e: Pointer) => {
      const { box, line } = handles()
      const { offsetX: x, offsetY: y } = e
      if (line) {
        const i = line.findIndex((p) => Math.hypot(view.x + p.x * view.zoom - x, view.y + p.y * view.zoom - y) <= HANDLE / 2 + 1)
        return i < 0 ? undefined : `end${i}`
      }
      if (!box) return undefined
      const l = view.x + box.x * view.zoom
      const t = view.y + box.y * view.zoom
      const r = l + box.w * view.zoom
      const b = t + box.h * view.zoom
      const near = (v: number, a: number, d: number) => Math.abs(v - a) <= d
      const within = (v: number, a: number, c: number) => v >= a - EDGE && v <= c + EDGE
      const v = !box.h ? '' : near(y, t, HANDLE / 2) ? 'n' : near(y, b, HANDLE / 2) ? 's' : ''
      const h = !box.w ? '' : near(x, l, HANDLE / 2) ? 'w' : near(x, r, HANDLE / 2) ? 'e' : ''
      if (v && h) return v + h
      if (!within(x, l, r) || !within(y, t, b)) return undefined
      if (box.h && near(y, t, EDGE)) return 'n'
      if (box.h && near(y, b, EDGE)) return 's'
      if (box.w && near(x, l, EDGE)) return 'w'
      if (box.w && near(x, r, EDGE)) return 'e'
      return undefined
    }
    const hit = (p: Point) => editor.engine.hit(editor.page.id, p.x, p.y, HIT / view.zoom)
    const track = () => {
      if (!pointer || drag || editor.pen) return
      canvas.style.cursor = portAt(pointer) ? 'pointer' : (CURSORS[handleAt(pointer) ?? ''] ?? '')
      const mode = pointer.ctrlKey ? 'deep' : 'click'
      const id = editor.tool === 'move' ? pick(editor.page.children, hit(toDoc(pointer)), editor.selection, mode) : undefined
      if (id !== hover) {
        hover = id
        redraw()
      }
    }

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
      track()
    }
    /** The frame of the edited text's thread on this page that `p` is inside. */
    const inEdited = (p: Point) => {
      const n = editor.editing && editor.nodes.get(editor.editing.id)?.node
      if (n?.kind !== 'text') return undefined
      const inside = (f: Node): f is TextNode =>
        f.kind === 'text' && f.story === n.story && p.x >= f.x && p.x <= f.x + f.w && p.y >= f.y && p.y <= f.y + f.h
      return inside(n) ? n : [...editor.nodes.values()].map((e) => e.node).find(inside)
    }
    const onPointerDown = (e: PointerEvent) => {
      if (e.button !== 0 && e.button !== 1) return
      const p = toDoc(e)
      const edited = e.button === 0 && !space ? inEdited(p) : undefined
      if (edited) {
        e.preventDefault()
        editor.dragging = true
        canvas.setPointerCapture(e.pointerId)
        const i = editor.engine.textIndex(edited.id, p.x, p.y)
        select(editor, e.shiftKey ? editor.editing!.anchor : i, i)
        drag = { kind: 'text' }
        return
      }
      editor.stopEditing()
      editor.dragging = true
      canvas.focus()
      canvas.setPointerCapture(e.pointerId)
      if (e.button === 1 || space) {
        e.preventDefault()
        drag = { kind: 'pan', last: { x: e.clientX, y: e.clientY } }
        canvas.dataset.panning = ''
        return
      }
      const parent = () => hit(p).findLast((id) => editor.nodes.get(id)?.node.kind === 'frame') ?? editor.page.id
      if (editor.threading) {
        // A loaded out-port threads into the text frame clicked, or a new one drawn.
        const from = editor.threading
        if (portAt(e)) return
        const target = hit(p).findLast((id) => editor.nodes.get(id)?.node.kind === 'text')
        if (target === from) return
        if (target) {
          try {
            editor.apply({ type: 'thread', from, to: target })
          } catch (err) {
            console.warn(err)
          }
          editor.set({ threading: null, selection: [target] })
          return
        }
        editor.apply({ type: 'beginUndoGroup' })
        const [id] = editor.apply({ type: 'create', parent: parent(), kind: 'text', x: p.x, y: p.y, w: 0, h: 0 })
        drag = { kind: 'draw', start: p, id, moved: false, tool: 'text', thread: from }
        editor.set({ selection: [id] })
        return
      }
      if (editor.tool === 'pen') {
        const pen = editor.pen
        const anchor = { x: p.x, y: p.y, hx: 0, hy: 0 }
        drag = { kind: 'pen', start: p }
        if (!pen) {
          editor.apply({ type: 'beginUndoGroup' })
          const [id] = editor.apply({ type: 'create', parent: parent(), kind: 'path', x: p.x, y: p.y, w: 0, h: 0 })
          editor.set({ pen: { id, anchors: [anchor] }, selection: [] })
          return
        }
        const first = pen.anchors[0]
        if (pen.anchors.length > 1 && Math.hypot(first.x - p.x, first.y - p.y) * view.zoom <= HANDLE) {
          drag = null
          editor.finishPen(true)
          return
        }
        const anchors = [...pen.anchors, anchor]
        editor.set({ pen: { ...pen, anchors } })
        editor.apply({ type: 'setPath', id: pen.id, path: penPath(anchors, false) })
        return
      }
      if (editor.tool !== 'move') {
        const tool = editor.tool
        editor.apply({ type: 'beginUndoGroup' })
        const [id] = editor.apply({ type: 'create', parent: parent(), kind: tool, x: p.x, y: p.y, w: 0, h: 0 })
        drag = { kind: 'draw', start: p, id, moved: false, tool }
        editor.set({ selection: [id] })
        return
      }
      // Ctrl+Shift+click on a master layer that no page layer covers overrides it, as in InDesign.
      const master = (e.ctrlKey || e.metaKey) && e.shiftKey && !hit(p).length
        ? editor.engine.masterHit(editor.page.id, p.x, p.y, HIT / view.zoom)
        : undefined
      if (master) {
        editor.set({ selection: editor.apply({ type: 'override', page: editor.page.id, id: master }) })
        return
      }
      const port = portAt(e)
      if (port) {
        if (port.side === 'out') editor.set({ threading: port.node.id })
        drag = null
        return
      }
      const handle = handleAt(e)
      const line = handle?.startsWith('end') && ends(editor.selected()[0])
      if (line) {
        drag = { kind: 'end', start: p, id: editor.selection[0], ends: line, index: Number(handle!.slice(3)) }
        editor.apply({ type: 'beginUndoGroup' })
        return
      }
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
      const frames = editor.selected()
      const parents = new Set(frames.map((n) => editor.nodes.get(n.id)?.parent))
      const [flow] = parents
      const inFlow = parents.size === 1 && flow?.kind === 'frame' && flow.direction !== 'none' && frames.every((n) => !n.absolute)
      drag = { kind: 'move', start: p, frames, active: false, flow: inFlow ? flow : undefined }
    }
    const onPointerMove = (e: PointerEvent) => {
      const p = toDoc(e)
      pointer = { offsetX: e.offsetX, offsetY: e.offsetY, ctrlKey: e.ctrlKey || e.metaKey }
      if (editor.pen && !drag) {
        cursor = p
        redraw()
        return
      }
      if (!drag) return track()
      if (drag.kind === 'text') {
        const ed = editor.editing
        if (ed) select(editor, ed.anchor, editor.engine.textIndex(inEdited(p)?.id ?? ed.id, p.x, p.y))
      } else if (drag.kind === 'pan') {
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
      } else if (drag.kind === 'pen') {
        const pen = editor.pen
        if (!pen || Math.hypot(p.x - drag.start.x, p.y - drag.start.y) * view.zoom <= DRAG) return
        const anchors = [...pen.anchors]
        anchors[anchors.length - 1] = { ...anchors.at(-1)!, hx: p.x - drag.start.x, hy: p.y - drag.start.y }
        editor.set({ pen: { ...pen, anchors } })
        editor.apply({ type: 'setPath', id: pen.id, path: penPath(anchors, false) })
      } else if (drag.kind === 'draw') {
        drag.moved ||= Math.hypot(p.x - drag.start.x, p.y - drag.start.y) * view.zoom > DRAG
        if (!drag.moved) return
        let dx = p.x - drag.start.x
        let dy = p.y - drag.start.y
        if (drag.tool === 'line' || drag.tool === 'arrow') {
          if (e.shiftKey) [dx, dy] = snap45(dx, dy)
          const { x, y } = drag.start
          editor.apply({ type: 'setPath', id: drag.id, path: [0, x, y, 1, x + dx, y + dy] })
          return
        }
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
          if (e.altKey) {
            editor.set({ selection: editor.apply({ type: 'duplicate', ids: drag.frames.map((n) => n.id) }) })
            drag.frames = editor.selected()
          }
        }
        if (drag.flow) {
          drag.to = insertion(drag.flow, drag.frames.map((n) => n.id), p)
          redraw()
          return
        }
        if (e.shiftKey) {
          if (Math.abs(dx) > Math.abs(dy)) dy = 0
          else dx = 0
        }
        for (const n of drag.frames) editor.apply({ type: 'setFrame', id: n.id, x: n.x + dx, y: n.y + dy, w: n.w, h: n.h })
      } else if (drag.kind === 'end') {
        const fixed = drag.ends[1 - drag.index]
        const from = drag.ends[drag.index]
        let dx = from.x + p.x - drag.start.x - fixed.x
        let dy = from.y + p.y - drag.start.y - fixed.y
        if (e.shiftKey) [dx, dy] = snap45(dx, dy)
        const moved = { x: fixed.x + dx, y: fixed.y + dy }
        const [a, b] = drag.index ? [fixed, moved] : [moved, fixed]
        editor.apply({ type: 'setPath', id: drag.id, path: [0, a.x, a.y, 1, b.x, b.y] })
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
          editor.apply({ type: 'setFrame', id: n.id, ...f, ignoreConstraints: e.ctrlKey || e.metaKey })
        }
      }
    }
    const onPointerUp = (e: PointerEvent) => {
      if (drag?.kind === 'draw' && drag.thread) {
        const from = editor.lookup(drag.thread)?.node
        if (!drag.moved && from) editor.apply({ type: 'setFrame', id: drag.id, x: drag.start.x, y: drag.start.y, w: from.w, h: from.h })
        try {
          editor.apply({ type: 'thread', from: drag.thread, to: drag.id })
        } catch (err) {
          console.warn(err)
        }
        editor.set({ threading: null, selection: [drag.id] })
      } else if (drag?.kind === 'draw') {
        if (drag.tool !== 'text' && !drag.moved) {
          const [w, h] = DEFAULT_SIZE[drag.tool]
          editor.apply({ type: 'setFrame', id: drag.id, x: drag.start.x, y: drag.start.y, w: w * MM, h: h * MM })
        }
        editor.setTool('move')
        const n = editor.nodes.get(drag.id)?.node
        if (n?.kind === 'text') editor.set({ editing: { id: n.id, anchor: 0, focus: n.text.length } })
      }
      if (drag?.kind === 'move' && drag.flow && drag.to) {
        editor.apply({ type: 'move', ids: drag.frames.map((n) => n.id), parent: drag.flow.id, index: drag.to.index })
      }
      if (drag?.kind === 'draw' || drag?.kind === 'resize' || drag?.kind === 'end' || (drag?.kind === 'move' && drag.active)) {
        editor.apply({ type: 'endUndoGroup' })
      }
      if (drag?.kind === 'move' && !drag.active && !e.shiftKey) {
        const id = pick(editor.page.children, hit(toDoc(e)), editor.selection, e.ctrlKey || e.metaKey ? 'deep' : 'click')
        if (id && editor.selection.length > 1) editor.set({ selection: [id] })
      }
      drag = null
      editor.dragging = false
      delete canvas.dataset.panning
      track()
      redraw()
    }
    const onDoubleClick = (e: MouseEvent) => {
      if (editor.tool !== 'move') return
      const port = portAt(e)
      if (port) {
        // Double-clicking a port breaks the thread there.
        const id = port.side === 'out' ? port.node.id : port.node.prev
        if (id && (port.side === 'in' || port.node.next)) editor.apply({ type: 'unthread', id })
        editor.set({ threading: null })
        return
      }
      const p = toDoc(e)
      const edited = inEdited(p)
      if (edited) {
        const [a, b] = wordAt(edited.text, editor.engine.textIndex(edited.id, p.x, p.y))
        select(editor, a, b)
        return
      }
      const id = pick(editor.page.children, hit(p), editor.selection, 'double')
      const n = id && editor.nodes.get(id)?.node
      if (n && n.kind === 'text' && editor.selection.includes(id)) {
        const i = editor.engine.textIndex(n.id, p.x, p.y)
        editor.set({ editing: { id: n.id, anchor: i, focus: i } })
      } else if (id) editor.set({ selection: [id] })
    }
    /** A triple click in the edited text selects all of it. */
    const onClick = (e: MouseEvent) => {
      const edited = e.detail >= 3 && editor.tool === 'move' ? inEdited(toDoc(e)) : undefined
      if (edited) select(editor, 0, edited.text.length)
    }
    const onLeave = () => {
      pointer = undefined
      hover = undefined
      cursor = undefined
      redraw()
    }
    const onKey = (e: KeyboardEvent) => {
      if (isTyping(e)) return
      if ((e.key === 'Control' || e.key === 'Meta') && pointer) {
        pointer.ctrlKey = e.type === 'keydown'
        track()
      }
      if (e.code === 'Space' && (e.target === canvas || e.target === document.body)) {
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
      track()
      e.preventDefault()
    }

    const unsubscribe = editor.subscribe(() => {
      if (editor.page.id !== shown) {
        views.set(shown, { ...view })
        shown = editor.page.id
        const kept = views.get(shown)
        if (kept) {
          Object.assign(view, kept)
          setZoom(view.zoom)
        } else fit()
      }
      wake()
      canvas.dataset.tool = editor.tool
      canvas.toggleAttribute('data-threading', editor.threading !== null)
      track()
      redraw()
    })
    canvas.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('pointercancel', onPointerUp)
    canvas.addEventListener('pointerleave', onLeave)
    canvas.addEventListener('dblclick', onDoubleClick)
    canvas.addEventListener('click', onClick)
    window.addEventListener('keydown', onKey)
    window.addEventListener('keyup', onKey)
    return () => {
      cancelAnimationFrame(frame)
      clearInterval(blink)
      unsubscribe()
      resize.disconnect()
      canvas.removeEventListener('wheel', onWheel)
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.removeEventListener('pointercancel', onPointerUp)
      canvas.removeEventListener('pointerleave', onLeave)
      canvas.removeEventListener('dblclick', onDoubleClick)
      canvas.removeEventListener('click', onClick)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('keyup', onKey)
      renderer.delete()
      surface?.delete()
    }
  }, [ck, editor])

  return (
    <div className="stage">
      <canvas ref={ref} className="canvas" aria-label="Page canvas" tabIndex={-1} data-tool="move" />
      {editing && (
        <textarea
          ref={(el) => {
            area.current = el
            el?.focus({ preventScroll: true })
          }}
          className="text-input"
          aria-label="Text editor"
          data-composing={composing || undefined}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          onBlur={(e) => {
            if (editor.editing && e.relatedTarget === null) e.currentTarget.focus({ preventScroll: true })
          }}
          onKeyDown={(e) => {
            if (e.nativeEvent.isComposing) return
            if (handleTextKey(editor, e.nativeEvent)) e.preventDefault()
          }}
          onInput={(e) => {
            if ((e.nativeEvent as InputEvent).isComposing) return
            const v = e.currentTarget.value
            e.currentTarget.value = ''
            insert(editor, v)
          }}
          onCompositionStart={() => setComposing(true)}
          onCompositionEnd={(e) => {
            setComposing(false)
            e.currentTarget.value = ''
            insert(editor, e.data)
          }}
          onCopy={(e) => {
            const [a, b] = range(editor.editing!)
            e.clipboardData.setData('text/plain', textOf(editor).slice(a, b))
            e.preventDefault()
          }}
          onCut={(e) => {
            const [a, b] = range(editor.editing!)
            e.clipboardData.setData('text/plain', textOf(editor).slice(a, b))
            insert(editor, '')
            e.preventDefault()
          }}
          onPaste={(e) => {
            insert(editor, e.clipboardData.getData('text/plain').replace(/\r\n?/g, '\n'))
            e.preventDefault()
          }}
        />
      )}
      <output className="zoom" aria-label="Zoom">
        {Math.round((zoom / PX_PER_PT) * 100)}%
      </output>
    </div>
  )
}
