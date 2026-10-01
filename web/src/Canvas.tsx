import { useEffect, useRef, useState } from 'react'
import type { CanvasKit, Surface } from 'canvaskit-wasm'
import { MM, bounds, ends, insertion, useEditor, type Editor, type Point, type Tool } from './editor'
import type { Container, NewKind, Node, Page, TextNode } from './model'
import { radii } from './model'
import { penPath } from './pen'
import { handleAt, portAt, portsOf, radiusHandles, rect, resized, spin, upright } from './handles'
import { Renderer, fitView, HANDLE, type Box, type View } from './renderer'
import { pick } from './select'
import { length, settings, subscribeSettings, UNITS } from './settings'
import { equals, guides, measure, nearest, snap, spacings, targets, type Guide, type Lines, type Measure } from './snap'
import { nearest as nearestSegment, remove, shift, split, type At, type Contour, type Knot } from './vector'
import { handleTextKey, insert, range, select, textOf, wordAt } from './textEdit'
import { Switcher } from './Switcher'
import { Quick } from './Quick'

const PX_PER_PT = 96 / 72
const DRAG = 3
const HIT = 6
/** Distance in px within which layers snap. */
const SNAP = 5
const TURN = 'M6 18a12 12 0 0 1 12-12M3 15l3 3 3-3M15 3l3 3-3 3'
const ARROW = 'M4 12h16M7 9l-3 3 3 3M17 9l3 3-3 3'
/** Degrees counterclockwise of the arrow of each resize handle and of the turn arrow at each corner. */
const RESIZE: Record<string, number> = { e: 0, ne: 45, n: 90, nw: 135, w: 180, sw: 225, s: 270, se: 315 }
const TURNS: Record<string, number> = { nw: 0, sw: 90, se: 180, ne: 270 }
const NATIVE = ['ew-resize', 'nesw-resize', 'ns-resize', 'nwse-resize']

/** The cursor over the handle `handle` of a selection turned `turn` degrees counterclockwise. */
function cursorOf(handle: string, turn: number) {
  const svg = (path: string, degrees: number, fallback: string) =>
    `url("data:image/svg+xml,${encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke-linecap="round" stroke-linejoin="round">
      <g transform="rotate(${-Math.round(degrees)} 12 12)"><path d="${path}" stroke="#000" stroke-width="3.5"/><path d="${path}" stroke="#fff" stroke-width="1.5"/></g></svg>`,
    )}") 12 12, ${fallback}`
  if (handle.startsWith('rotate')) return svg(TURN, TURNS[handle.slice(6)] + turn, 'crosshair')
  if (!(handle in RESIZE)) return handle.startsWith('end') ? 'crosshair' : ''
  const a = (((RESIZE[handle] + turn) % 180) + 180) % 180
  const k = Math.round(a / 45)
  return Math.abs(a - k * 45) < 1 ? NATIVE[k % 4] : svg(ARROW, a, NATIVE[k % 4])
}
/** Size in mm of a layer made with a click; a clicked text is auto width, a dragged one fixed. */
const DEFAULT_SIZE: Record<Exclude<Tool, 'move' | 'pen'>, [number, number]> = {
  rect: [30, 30], ellipse: [30, 30], polygon: [30, 30], star: [30, 30], frame: [30, 30], text: [0, 0],
  line: [30, 0],
}

type Pointer = { offsetX: number; offsetY: number; ctrlKey: boolean }
/** What a drag snaps to and the layers it measures to. */
type Snaps = { lines: Lines; others: Box[] }
type Drag =
  | { kind: 'pan'; last: Point }
  | { kind: 'move'; start: Point; frames: Node[]; active: boolean; flow?: Container; to?: ReturnType<typeof insertion>; box?: Box; snaps?: Snaps }
  | { kind: 'resize'; start: Point; handle: string; box: Box; frames: Node[]; snaps: Snaps; by: number; turn: number; own: number }
  | { kind: 'rotate'; c: Point; from: number; nodes: Node[]; handle: string; turn: number }
  | { kind: 'end'; start: Point; id: string; ends: [Point, Point]; index: number }
  | { kind: 'radius'; start: Point; id: string; box: Box; radii: number[]; index: number; turn: number }
  | { kind: 'marquee'; start: Point; end: Point; base: string[] }
  | { kind: 'draw'; start: Point; id: string; dx: number; moved: boolean; tool: keyof typeof DEFAULT_SIZE; thread?: string; snaps?: Snaps }
  | { kind: 'pen'; start: Point }
  | { kind: 'knot'; start: Point; cs: Contour[]; at: At; part: 'point' | 'in' | 'out' }
  | { kind: 'text' }
  | { kind: 'swap'; id: string; at: Point }

/** Snaps a vector to the nearest multiple of 45°. */
/** Degrees counterclockwise of `p` around `c`. */
function angle(c: Point, p: Point) {
  return (Math.atan2(c.y - p.y, p.x - c.x) * 180) / Math.PI
}

function snap45(dx: number, dy: number) {
  const a = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4)
  const d = Math.hypot(dx, dy)
  return [Math.cos(a) * d, Math.sin(a) * d]
}

export function isTyping(e: Event) {
  return e.target instanceof HTMLElement && e.target.closest('input:not([type=checkbox]), textarea, select, [contenteditable]') !== null
}

const RULER = 20

/**
 * Draws a ruler whose 0 is at `origin` px, at `scale` px per unit, with the span
 * `extent` in px shaded and a mark at each of `guides` in px.
 */
function drawRuler(c: HTMLCanvasElement, horizontal: boolean, origin: number, scale: number, extent?: [number, number], guides: number[] = []) {
  const { clientWidth: w, clientHeight: h } = c
  if (!w || !h) return
  const dpr = devicePixelRatio
  if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) [c.width, c.height] = [Math.round(w * dpr), Math.round(h * dpr)]
  const g = c.getContext('2d')!
  g.setTransform(dpr, 0, 0, dpr, 0, 0)
  g.clearRect(0, 0, w, h)
  const at = (p: number, from: number, to: number) => (horizontal ? g.rect(p, from, 1, to - from) : g.rect(from, p, to - from, 1))
  const style = getComputedStyle(c)
  g.fillStyle = style.color
  if (extent) {
    g.globalAlpha = 0.14
    g.beginPath()
    if (horizontal) g.rect(extent[0], 0, extent[1] - extent[0], RULER)
    else g.rect(0, extent[0], RULER, extent[1] - extent[0])
    g.fill()
  }
  const step = [0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, 200].find((s) => s * scale >= 7) ?? 500
  const major = [0.1, 1, 10, 100].includes(step) ? 10 : 5
  const first = Math.floor(-origin / scale / step)
  const end = (len: number) => Math.ceil((len - origin) / scale / step)
  g.globalAlpha = 0.45
  g.beginPath()
  const len = horizontal ? w : h
  for (let n = first; n < end(len); n++) {
    const t = n % major === 0 ? 9 : n % 5 === 0 ? 5 : 3
    at(Math.round(n * step * scale + origin), RULER - t, RULER)
  }
  g.fill()
  g.globalAlpha = 1
  g.font = "500 9px 'Hanken Grotesk', system-ui, sans-serif"
  for (let n = Math.ceil(first / major) * major; n < end(len); n += major) {
    const v = Math.round(n * step * 10) / 10
    const p = Math.round(n * step * scale + origin)
    if (horizontal) g.fillText(String(v), p + 3, 9)
    else {
      g.save()
      g.translate(9, p - 3)
      g.rotate(-Math.PI / 2)
      g.fillText(String(v), 0, 0)
      g.restore()
    }
  }
  g.fillStyle = style.getPropertyValue('--magenta')
  g.beginPath()
  for (const p of guides) at(Math.round(p) - 0.5, 0, RULER)
  g.fill()
}

/** Half a blink period of the caret in ms. */
const BLINK = 530

export function Canvas({ ck, editor }: { ck: CanvasKit; editor: Editor }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const rulerX = useRef<HTMLCanvasElement>(null)
  const rulerY = useRef<HTMLCanvasElement>(null)
  const area = useRef<HTMLTextAreaElement>(null)
  const quick = useRef<HTMLDivElement>(null)
  const marks = useRef<SVGSVGElement>(null)
  const [zoom, setZoom] = useState(0)
  const [composing, setComposing] = useState(false)
  const editing = useEditor(editor, (e) => e.editing !== null)

  useEffect(() => {
    const canvas = ref.current!
    const renderer = new Renderer(ck, editor.engine)
    const view: View = { x: 0, y: 0, zoom: 1 }
    /** The view of each spread left for another, as Figma keeps it for pages. */
    const spreadKey = () => `${editor.spread.map((p) => p.id).join()}/${editor.sheets.map((s) => `${s.width}x${s.height}`).join()}`
    let shown = spreadKey()
    const gl = ck.MakeWebGLContext(ck.GetWebGLContext(canvas))!
    let surface: Surface | null = null
    let frame = 0
    let fitted = false
    let space = false
    let drag: Drag | null = null
    let hover: string | undefined
    let cursor: Point | undefined
    /** Last pointer position and Ctrl state over the canvas, for hover and cursor. */
    let pointer: Pointer | undefined
    let port: 'in' | 'out' | undefined
    let alt = false
    let snapped: Guide[] = []
    let measures: Measure[] = []
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

    /** A point in the space of the spread, whose spine is at x 0. */
    const toDoc = (e: { offsetX: number; offsetY: number }): Point => ({
      x: (e.offsetX - view.x) / view.zoom,
      y: (e.offsetY - view.y) / view.zoom,
    })
    /** A layer where it sits on the spread. */
    const placed = <T extends Node>(n: T): T => ({ ...n, x: n.x + editor.dx(n.id) })
    /** `n` at the upright box it covers on the spread. */
    const covered = (n: Node): Node => {
      const [x, y, w, h] = n.bounds
      return { ...n, x: x + editor.dx(n.id), y, w, h }
    }
    /** The index in the text `id` nearest `p`. */
    const textAt = (id: string, p: Point) => {
      const q = editor.local(id, p)
      return editor.engine.textIndex(id, q.x, q.y)
    }
    /** The page of the spread under `p`, or the nearest. */
    const pageAt = (p: Point) => {
      const away = (q: Page) => Math.max(q.x - p.x, p.x - q.x - q.width, 0)
      return editor.spread.reduce((a, b) => (away(b) < away(a) ? b : a))
    }
    /** The path of layers at `p` on the topmost page of the spread that has one there. */
    /** The text frame under `p`, innermost first. */
    const textUnder = (p: Point) => hit(p).path.findLast((id) => editor.nodes.get(id)?.node.kind === 'text')
    const hit = (p: Point) => {
      for (const page of [...editor.spread].reverse()) {
        const path = editor.engine.hit(page.id, p.x - page.x, p.y, HIT / view.zoom)
        if (path.length) return { page, path }
      }
      return { page: pageAt(p), path: [] as string[] }
    }
    /** The inks of the page `p` from the preflight, over its bleed. */
    const inkedOf = (p: Page) => {
      const v = editor.previewed
      const s = v?.sheets.find((s) => s.page === p.id)
      return s && { ...s, rect: { x: -p.bleed, y: -p.bleed, w: (s.width * 72) / v!.ppi, h: (s.height * 72) / v!.ppi } }
    }
    /** The ink coverage in % of the preflight at `p`. */
    const inkAt = (p: Point) => {
      const page = pageAt(p)
      const s = inkedOf(page)
      if (!s) return null
      const [x, y] = [p.x - page.x - s.rect.x, p.y - s.rect.y].map((v) => Math.floor((v / s.rect.w) * s.width))
      return x >= 0 && y >= 0 && x < s.width && y < s.height ? Math.round(s.coverage[y * s.width + x]) : null
    }
    /** The layer that a click at `p` picks. */
    const pickAt = (p: Point, mode: 'click' | 'double' | 'deep') => {
      const { page, path } = hit(p)
      return pick(page.children, path, editor.selection, mode)
    }
    /**
     * The out-port in screen space of a single selected text frame that can thread, as in
     * InDesign, on its right edge above the bottom, and its in-port below the top of the
     * left edge if a frame threads into it.
     */
    const ports = () => {
      const [n] = editor.selected()
      if (editor.tool !== 'move' || editor.selection.length !== 1 || editor.editing || n?.kind !== 'text') return undefined
      if (n.sizing.horizontal === 'hug' && n.sizing.vertical === 'hug' && !n.prev && !n.next) return undefined
      if (editor.shown(n).rotation) return undefined
      const p = portsOf(view, placed(n))
      return { node: n, out: p.out, in: n.prev ? p.in : undefined }
    }
    const portUnder = (e: Pointer) => {
      const p = ports()
      const side = p && portAt(p, e.offsetX, e.offsetY)
      return side && { side, node: p.node }
    }
    /** Box handles of the selection, or the ends of a single selected line. */
    const handles = () => {
      const nodes = editor.selected().map(placed)
      if ((editor.tool !== 'move' && drag?.kind !== 'draw') || !nodes.length || editor.editing || editor.vector) return {}
      const n = nodes.length === 1 ? nodes[0] : undefined
      const line = n && ends(n)
      if (line) return { line }
      const box: Box = n ? editor.shown(editor.selected()[0]) : bounds(nodes)
      return { box, radii: n?.kind === 'shape' && n.shape === 'rect' ? radiusHandles(view, box, radii(n)) : undefined }
    }

    /** What the layers `ids` snap to: the pages of the spread and the visible layers beside them. */
    const snapsNow = (ids = editor.selection): Snaps => {
      const top = (id: string) => {
        let e = editor.nodes.get(id)
        while (e?.parent) e = editor.nodes.get(e.parent.id)
        return e?.node.id
      }
      const skip = new Set([...ids, ...ids.map(top)])
      const siblings = ids.flatMap((id) => editor.nodes.get(id)?.parent?.children ?? []).map(covered)
      const tops = editor.spread.flatMap((page) => page.children.map(covered))
      const others = [...new Map([...tops, ...siblings].map((n) => [n.id, n])).values()].filter((n) => !n.hidden && !skip.has(n.id))
      return { lines: targets(editor.sheets, others, editor.grids), others }
    }
    /** `p` moved onto what it snaps to, unless `off`, with the guides through it. */
    const snapPoint = (p: Point, snaps: Snaps | undefined, off: boolean) => {
      if (!snaps || off) return { p, guides: [] }
      const q = { x: p.x + snap([p.x], snaps.lines.x, SNAP / view.zoom), y: p.y + snap([p.y], snaps.lines.y, SNAP / view.zoom) }
      return { p: q, guides: guides({ ...q, w: 0, h: 0 }, snaps.lines, [q.x], [q.y]) }
    }
    /** The centre of the image being placed at `p`, moved onto what it snaps to unless `off`, with the guides through it. */
    const placeAt = (p: Point, off: boolean) => {
      const { w, h } = editor.placing[0]
      const { lines } = snapsNow([])
      const [x, y] = [p.x - w / 2, p.y - h / 2]
      const dx = off ? 0 : snap([x, x + w / 2, x + w], lines.x, SNAP / view.zoom)
      const dy = off ? 0 : snap([y, y + h / 2, y + h], lines.y, SNAP / view.zoom)
      const [xs, ys] = [[x + dx, x + dx + w / 2, x + dx + w], [y + dy, y + dy + h / 2, y + dy + h]]
      return { p: { x: p.x + dx, y: p.y + dy }, guides: off ? [] : guides({ x: x + dx, y: y + dy, w, h }, lines, xs, ys) }
    }
    const svgOf = (target?: Box) => {
      const [X, Y] = [(x: number) => view.x + x * view.zoom, (y: number) => view.y + y * view.zoom]
      const line = (x1: number, y1: number, x2: number, y2: number, cls = '') =>
        `<line class="${cls}" x1="${X(x1)}" y1="${Y(y1)}" x2="${X(x2)}" y2="${Y(y2)}"/>`
      let svg = snapped.map((g) => (g.axis === 'x' ? line(g.at, g.from, g.at, g.to) : line(g.from, g.at, g.to, g.at))).join('')
      if (target) svg += `<rect class="target" x="${X(target.x)}" y="${Y(target.y)}" width="${target.w * view.zoom}" height="${target.h * view.zoom}"/>`
      const labels: Box[] = []
      let tags = ''
      for (const m of measures) {
        svg += line(m.x1, m.y1, m.x2, m.y2, m.dashed ? 'dashed' : '')
        if (m.dashed) continue
        const t = length(m.length)
        const w = t.length * 6.2 + 10
        const cx = m.y1 === m.y2 ? X((m.x1 + m.x2) / 2) : X(m.x1) + 9 + w / 2
        let cy = m.y1 === m.y2 ? Y(m.y1) + 13 : Y((m.y1 + m.y2) / 2)
        while (labels.some((l) => Math.abs(l.x - cx) < (l.w + w) / 2 && Math.abs(l.y - cy) < 20)) cy += 20
        labels.push({ x: cx, y: cy, w, h: 18 })
        tags += `<rect class="label" x="${cx - w / 2}" y="${cy - 9}" width="${w}" height="18" rx="4"/><text x="${cx}" y="${cy}">${t}</text>`
      }
      const moving = drag?.kind === 'swap' ? drag : undefined
      for (const s of swaps()) {
        const on = moving?.id === s.id
        svg += `<circle class="swap" cx="${X(on ? moving.at.x : s.x)}" cy="${Y(on ? moving.at.y : s.y)}" r="5"/>`
      }
      return svg + tags
    }

    /** Ports of the selected text frame; over a port, the lines of its thread on this spread and the frame the port links to. */
    const threadOverlay = () => {
      const p = drag ? undefined : ports()
      if (!p) return {}
      const n = p.node
      const link = port && editor.nodes.get((port === 'in' ? n.prev : n.next) ?? '')?.node
      const frames = [...editor.nodes.values()].flatMap(({ node }) => (node.kind === 'text' && node.story === n.story ? [node] : []))
      const lines = frames.flatMap((f) => {
        const next = f.next && frames.find((g) => g.id === f.next)
        return next ? [[portsOf(view, placed(f)).out, portsOf(view, placed(next)).in] as [Point, Point]] : []
      })
      return {
        ports: [
          ...(p.in ? [{ ...p.in, state: 'threaded' as const }] : []),
          { ...p.out, state: n.overset ? 'overset' : n.next ? 'threaded' : 'open' },
        ] as const,
        threads: port ? lines : [],
        ...(link && { hover: placed(link) }),
      }
    }

    const redraw = () => {
      canvas.dataset.sheets = JSON.stringify(editor.sheets.map(({ x, width, height, bleed }) => ({ x, width, height, bleed })))
      frame ||= requestAnimationFrame(paint)
    }
    const paint = () => {
      cancelAnimationFrame(frame)
      frame = 0
      if (!surface) return
      const { box, line, radii: corners } = drag?.kind === 'marquee' ? {} : handles()
      const h = editor.hover ?? hover
      const over = h && !editor.selection.includes(h) ? editor.nodes.get(h)?.node : undefined
      const outline = (n: Node) => ends(placed(n)) ?? editor.shown(n)
      const hovered = over && editor.shown(over)
      const marquee =
        drag?.kind === 'marquee'
          ? rect(
              { x: view.x + drag.start.x * view.zoom, y: view.y + drag.start.y * view.zoom },
              { x: view.x + drag.end.x * view.zoom, y: view.y + drag.end.y * view.zoom },
            )
          : undefined
      const ed = editor.editing
      let text: { ops: Uint32Array; x: number }[] | undefined
      if (ed) {
        const [cx, top, bottom] = editor.engine.caret(ed.id, ed.focus)
        const x = cx + editor.dx(ed.id)
        Object.assign(area.current?.style ?? {}, {
          left: `${view.x + x * view.zoom}px`,
          top: `${view.y + top * view.zoom}px`,
          height: `${(bottom - top) * view.zoom}px`,
        })
        if (caretOn || ed.anchor !== ed.focus) {
          text = editor.spread.map((p) => ({
            ops: editor.engine.textOverlay(ed.id, ed.anchor, ed.focus, 1 / view.zoom, p.id).slice(),
            x: p.x,
          }))
        }
      }
      const spread = editor.spread
      const pen = editor.pen
      const penDx = pen ? editor.dx(pen.id) : 0
      const flowDx = drag?.kind === 'move' && drag.flow ? editor.dx(drag.flow.id) : 0
      const insert = drag?.kind === 'move' ? drag.to?.line.map((q) => ({ x: q.x + flowDx, y: q.y })) : undefined
      const lists = spread.map((p) => ({ id: p.id, x: p.x, inks: editor.preflight ? inkedOf(p) : undefined }))
      const image = editor.placing[0]
      renderer.keepImages(Object.keys(editor.snapshot.images).join())
      renderer.draw(surface.getCanvas(), lists, editor.sheets, view, canvas.width / canvas.clientWidth, {
        text,
        accent: getComputedStyle(canvas).getPropertyValue('--accent'),
        selection: editor.selection.length > 1 || ed || drag?.kind === 'marquee' ? editor.selected().map(outline) : [],
        hover: image && cursor ? { x: cursor.x - image.w / 2, y: cursor.y - image.h / 2, w: image.w, h: image.h } : over && outline(over),
        marquee,
        handles: box,
        radii: corners,
        ends: line,
        pen: pen && { anchors: pen.anchors.map((a) => ({ ...a, x: a.x + penDx })), cursor: drag ? undefined : cursor },
        insert: insert as [Point, Point] | undefined,
        grids: editor.grids && !editor.preflight,
        vector: vector(),
        ...threadOverlay(),
      }, editor.snapshot.colorMode === 'cmyk', editor.preflight ? editor.inks.on : 15)
      surface.flush()
      const left = Math.min(...editor.sheets.map((s) => s.x))
      const sel = editor.selection.length ? bounds(editor.selected().map((n) => upright(editor.shown(n)))) : undefined
      const x0 = view.x + left * view.zoom
      const bar = quick.current!
      bar.hidden =
        !sel || !settings.quickEdit || (alt && !!pointer) || !!ed || !!pen || editor.threading !== null || editor.overview !== null || editor.preflight || editor.tool !== 'move' ||
        (!!drag && drag.kind !== 'pan' && !(drag.kind === 'move' && !drag.active))
      if (sel && !bar.hidden) {
        const { clientWidth: vw, clientHeight: vh } = canvas
        const w = bar.offsetWidth
        const h = bar.offsetHeight
        const top = view.y + sel.y * view.zoom - h - 14
        bar.style.left = `${Math.min(Math.max(view.x + (sel.x + sel.w / 2) * view.zoom - w / 2, 8), vw - w - 8)}px`
        bar.style.top = `${Math.min(Math.max(top >= 8 ? top : view.y + (sel.y + sel.h) * view.zoom + 14, 8), vh - h - 8)}px`
      }
      const page = pointer ? pageAt(toDoc(pointer)) : undefined
      const target = !alt || !sel || drag || ed || pen || editor.tool !== 'move' || editor.overview !== null
        ? undefined
        : hovered || (page && { x: page.x, y: 0, w: page.width, h: page.height })
      if (target) measures = measure(sel!, target, snapsNow().others)
      else if (!drag) measures = []
      marks.current!.innerHTML = svgOf(target)
      const at = (axis: 'x' | 'y', o: number) => snapped.filter((g) => g.axis === axis).map((g) => o + g.at * view.zoom)
      drawRuler(rulerX.current!, true, x0, view.zoom * UNITS[settings.unit], sel && [view.x + sel.x * view.zoom, view.x + (sel.x + sel.w) * view.zoom], at('x', view.x))
      drawRuler(rulerY.current!, false, view.y, view.zoom * UNITS[settings.unit], sel && [view.y + sel.y * view.zoom, view.y + (sel.y + sel.h) * view.zoom], at('y', view.y))
    }
    const show = (to: View) => {
      Object.assign(view, to)
      setZoom(view.zoom)
      redraw()
    }
    /** The view at `zoom` that keeps the point `px`, `py` where it is. */
    const around = (px: number, py: number, zoom: number): View => {
      zoom = Math.min(256, Math.max(0.02, zoom))
      return { x: px - ((px - view.x) * zoom) / view.zoom, y: py - ((py - view.y) * zoom) / view.zoom, zoom }
    }
    const zoomAt = (px: number, py: number, zoom: number) => show(around(px, py, zoom))
    const fit = (sheets = editor.sheets) => show(fitView(sheets, canvas.clientWidth, canvas.clientHeight))
    let anim = 0
    let target: View | undefined
    /** Moves the view to `to` in 180 ms, scaling about the point both views share; any input ends the move at `to`. */
    const glide = (to: View) => {
      settle()
      if (matchMedia('(prefers-reduced-motion: reduce)').matches) return show(to)
      const from = { ...view }
      const start = performance.now()
      target = to
      const step = (t: number) => {
        const k = Math.min(1, (t - start) / 180)
        const e = 1 - (1 - k) ** 3
        const zoom = from.zoom * (to.zoom / from.zoom) ** e
        const f = to.zoom === from.zoom ? e : (zoom - from.zoom) / (to.zoom - from.zoom)
        show({ x: from.x + (to.x - from.x) * f, y: from.y + (to.y - from.y) * f, zoom })
        if (k < 1) anim = requestAnimationFrame(step)
        else target = undefined
      }
      anim = requestAnimationFrame(step)
    }
    const settle = () => {
      cancelAnimationFrame(anim)
      if (target) show(target)
      target = undefined
    }

    /** Whether a click at `p` with the pen closes its path on the first anchor. */
    const closes = (p: Point) => {
      const pen = editor.pen
      const first = pen?.anchors[0]
      return !!first && pen.anchors.length > 1 && Math.hypot(first.x - p.x + editor.dx(pen.id), first.y - p.y) * view.zoom <= HANDLE
    }
    /** The knots of the path being edited on the spread, and the one picked. */
    const vector = () => {
      const v = editor.vector
      if (!v) return undefined
      const dx = editor.dx(v.id)
      const contours = editor.knots().map((c) => ({ ...c, knots: c.knots.map((k) => ({ ...k, x: k.x + dx, ix: k.ix + dx, ox: k.ox + dx })) }))
      return { contours, at: v.at }
    }
    /** The centres of several selected layers; one dragged onto another swaps their places. */
    const swaps = () =>
      editor.tool !== 'move' || editor.selection.length < 2 || editor.editing || editor.vector
        ? []
        : editor.selected().map((n) => {
            const b = upright(editor.shown(n))
            return { id: n.id, box: b, x: b.x + b.w / 2, y: b.y + b.h / 2 }
          })
    /** A selected layer under `p` that a mask hides there. */
    const maskedAt = (p: Point) =>
      editor.selected().find((n) => {
        const { parent, page } = editor.nodes.get(n.id)!
        const nodes = (parent ?? page)!.children
        const b = upright(editor.shown(n))
        return nodes.slice(nodes.indexOf(n) + 1).some((m) => m.mask && !m.hidden) && p.x >= b.x && p.x <= b.x + b.w && p.y >= b.y && p.y <= b.y + b.h
      })?.id
    /** Swaps the places of the layers `a` and `b`, in the order of an auto layout frame they share. */
    const swap = (a: string, b: string) => {
      const [p, q] = [editor.nodes.get(a)!, editor.nodes.get(b)!]
      const flow = p.parent === q.parent && p.parent?.kind === 'frame' && p.parent.direction !== 'none' ? p.parent : undefined
      editor.beginGroup()
      if (flow) {
        const [i, j] = [p.node, q.node].map((n) => flow.children.indexOf(n)).sort((m, n) => m - n)
        const [lo, hi] = [flow.children[i].id, flow.children[j].id]
        editor.apply({ type: 'move', ids: [hi], parent: flow.id, index: i })
        editor.apply({ type: 'move', ids: [lo], parent: flow.id, index: j })
      } else {
        const [m, n] = [p.node, q.node]
        editor.apply({ type: 'setFrame', id: m.id, x: n.x, y: n.y, w: m.w, h: m.h })
        editor.apply({ type: 'setFrame', id: n.id, x: m.x, y: m.y, w: n.w, h: n.h })
      }
      editor.endGroup()
    }
    const handleUnder = (e: Pointer) => handleAt(view, e.offsetX, e.offsetY, handles())
    const track = () => {
      editor.pointer = pointer && toDoc(pointer)
      if (!pointer || drag || editor.pen) return
      const side = portUnder(pointer)?.side
      const to = editor.threading && textUnder(toDoc(pointer))
      canvas.style.cursor = side ? 'pointer'
        : to ? (editor.engine.canThread(editor.threading!, to) ? '' : 'not-allowed')
        : editor.vector?.mode === 'add' ? 'crosshair' : cursorOf(handleUnder(pointer) ?? '', handles().box?.rotation ?? 0)
      const mode = pointer.ctrlKey ? 'deep' : 'click'
      const id = editor.tool === 'move' ? pickAt(toDoc(pointer), mode) : undefined
      if (id !== hover || side !== port) {
        hover = id
        port = side
        redraw()
      }
    }

    let place = canvas.getBoundingClientRect()
    const resize = new ResizeObserver(([entry]) => {
      const r = canvas.getBoundingClientRect()
      view.x += place.left - r.left
      view.y += place.top - r.top
      place = r
      const box = entry.devicePixelContentBoxSize?.[0]
      canvas.width = box ? box.inlineSize : Math.round(entry.contentRect.width * devicePixelRatio)
      canvas.height = box ? box.blockSize : Math.round(entry.contentRect.height * devicePixelRatio)
      surface?.delete()
      surface = ck.MakeOnScreenGLSurface(gl, canvas.width, canvas.height, ck.ColorSpace.SRGB)
      if (!fitted) {
        fitted = true
        fit()
      }
      paint()
    })
    try {
      resize.observe(canvas, { box: 'device-pixel-content-box' })
    } catch {
      resize.observe(canvas)
    }

    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      settle()
      const scale = e.deltaMode === 1 ? 16 : 1
      if (e.ctrlKey || e.metaKey) {
        const r = canvas.getBoundingClientRect()
        zoomAt(e.clientX - r.left, e.clientY - r.top, view.zoom * Math.exp(-e.deltaY * scale * 0.01))
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
      const inside = (f: Node): f is TextNode => {
        const x = f.x + editor.dx(f.id)
        return f.kind === 'text' && f.story === n.story && p.x >= x && p.x <= x + f.w && p.y >= f.y && p.y <= f.y + f.h
      }
      return inside(n) ? n : [...editor.nodes.values()].map((e) => e.node).find(inside)
    }
    /** The frame or page under `p` that a new layer goes into, and how far its page sits right of the spine. */
    const parentAt = (p: Point) => {
      const h = hit(p)
      const frame = h.path.findLast((id) => editor.nodes.get(id)?.node.kind === 'frame')
      const page = frame ? h.page : pageAt(p)
      return { parent: frame ?? page.id, dx: page.x }
    }
    const create = (kind: NewKind, p: Point) => {
      const { parent, dx } = parentAt(p)
      const [id] = editor.apply({ type: 'create', parent, kind, x: p.x - dx, y: p.y, w: 0, h: 0 })
      return { id, dx }
    }
    /** Starts to draw with `drag.tool`, which a click beside the pages leaves undone. */
    const draw = (drag: Extract<Drag, { kind: 'draw' }>) => {
      editor.beginGroup()
      Object.assign(drag, create(drag.tool, drag.start))
      editor.set({ selection: [drag.id] })
    }
    const onPointerDown = (e: PointerEvent) => {
      settle()
      let p = toDoc(e)
      if (e.button === 2 && editor.tool === 'move' && !editor.editing) {
        const id = pickAt(p, 'click')
        if (!id || !editor.selection.includes(id)) editor.set({ selection: id ? [id] : [] })
      }
      if (e.button !== 0 && e.button !== 1) return
      const edited = e.button === 0 && !space ? inEdited(p) : undefined
      if (edited) {
        e.preventDefault()
        editor.dragging = true
        canvas.setPointerCapture(e.pointerId)
        const i = textAt(edited.id, p)
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
      const image = editor.placing[0]
      if (image) {
        const at = placeAt(p, e.ctrlKey || e.metaKey).p
        const { parent, dx } = parentAt(at)
        const { hash, name, w, h } = image
        snapped = []
        const placed = editor.apply({ type: 'placeImage', parent, image: hash, name, x: at.x - dx - w / 2, y: at.y - h / 2, w, h })
        editor.set({ selection: placed, placing: editor.placing.slice(1) })
        return
      }
      if (editor.threading) {
        // A loaded out-port threads into the text frame clicked, or a new one drawn.
        const from = editor.threading
        if (portUnder(e)) return
        const to = textUnder(p)
        if (to === from) return
        if (to) {
          const ok = editor.engine.canThread(from, to)
          editor.apply({ type: 'thread', from, to })
          if (ok) editor.set({ threading: null, selection: [to] })
          return
        }
        editor.beginGroup()
        const { id, dx } = create('text', p)
        drag = { kind: 'draw', start: p, id, dx, moved: false, tool: 'text', thread: from }
        editor.set({ selection: [id] })
        return
      }
      if (editor.tool === 'pen') {
        const pen = editor.pen
        drag = { kind: 'pen', start: p }
        if (!pen) {
          editor.beginGroup()
          const { id, dx } = create('path', p)
          editor.set({ pen: { id, anchors: [{ x: p.x - dx, y: p.y, hx: 0, hy: 0 }] }, selection: [] })
          return
        }
        // Anchors are in the space of the path's page.
        const anchor = { x: p.x - editor.dx(pen.id), y: p.y, hx: 0, hy: 0 }
        if (closes(p)) {
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
        const snaps = snapsNow()
        p = snapPoint(p, snaps, e.ctrlKey || e.metaKey).p
        drag = { kind: 'draw', start: p, id: '', dx: 0, moved: false, tool, snaps }
        if (editor.sheets.some((q) => p.x >= q.x - q.bleed && p.x <= q.x + q.width + q.bleed && p.y >= -q.bleed && p.y <= q.height + q.bleed)) draw(drag)
        return
      }
      const v = editor.vector
      if (v) {
        const cs = editor.knots()
        const q = { x: p.x - editor.dx(v.id), y: p.y }
        const r = HIT / view.zoom
        const on = (x: number, y: number, k: Knot) => Math.hypot(x - q.x, y - q.y) <= r && (x !== k.x || y !== k.y)
        const k = v.at && cs[v.at[0]]?.knots[v.at[1]]
        let under: { at: At; part: 'point' | 'in' | 'out' } | undefined =
          k && v.at ? (on(k.ix, k.iy, k) ? { at: v.at, part: 'in' } : on(k.ox, k.oy, k) ? { at: v.at, part: 'out' } : undefined) : undefined
        cs.forEach(({ knots }, c) => knots.forEach((k, i) => {
          if (!under && Math.hypot(k.x - q.x, k.y - q.y) <= r) under = { at: [c, i], part: 'point' }
        }))
        const segment = nearestSegment(cs, q)
        if (v.mode === 'delete') {
          if (under?.part === 'point') editor.setKnots(remove(cs, under.at))
          drag = null
          return
        }
        if (v.mode === 'add' && !under && segment.d <= r) {
          const at: At = [segment.at[0], segment.at[1] + 1]
          const next = split(cs, segment.at, segment.t)
          editor.beginGroup()
          editor.setKnots(next, at)
          drag = { kind: 'knot', start: p, cs: next, at, part: 'point' }
          return
        }
        if (under) {
          editor.beginGroup()
          editor.set({ vector: { ...v, at: under.at } })
          drag = { kind: 'knot', start: p, cs, ...under }
          return
        }
        if (segment.d <= r) {
          editor.set({ vector: { ...v, at: null } })
          drag = null
          return
        }
        editor.set({ vector: null })
      }
      // Ctrl+Shift+click on a master layer that no page layer covers overrides it, as in InDesign.
      const under = pageAt(p)
      const master = (e.ctrlKey || e.metaKey) && e.shiftKey && !hit(p).path.length
        ? editor.engine.masterHit(under.id, p.x - under.x, p.y, HIT / view.zoom)
        : undefined
      if (master) {
        editor.set({ selection: editor.apply({ type: 'override', page: under.id, id: master }) })
        return
      }
      const port = portUnder(e)
      if (port) {
        if (port.side === 'out') editor.set({ threading: port.node.id })
        drag = null
        return
      }
      const centre = swaps().find((s) => Math.hypot(s.x - p.x, s.y - p.y) * view.zoom <= HANDLE)
      if (centre) {
        drag = { kind: 'swap', id: centre.id, at: p }
        return
      }
      const handle = handleUnder(e)
      const line = handle?.startsWith('end') && ends(placed(editor.selected()[0]))
      if (line) {
        drag = { kind: 'end', start: p, id: editor.selection[0], ends: line, index: Number(handle!.slice(3)) }
        editor.beginGroup()
        return
      }
      if (handle?.startsWith('radius')) {
        const n = placed(editor.selected()[0])
        const turn = editor.shown(n).rotation!
        if (n.kind === 'shape' && n.shape === 'rect') drag = { kind: 'radius', start: p, id: n.id, box: n, radii: radii(n), index: Number(handle.slice(6)), turn }
        editor.beginGroup()
        return
      }
      if (handle?.startsWith('rotate')) {
        const nodes = editor.selected()
        const b: Box = nodes.length === 1 ? editor.shown(nodes[0]) : bounds(nodes.map(placed))
        const c = { x: b.x + b.w / 2, y: b.y + b.h / 2 }
        drag = { kind: 'rotate', c, from: angle(c, p), nodes, handle, turn: b.rotation ?? 0 }
        editor.beginGroup()
        return
      }
      if (handle) {
        const frames = editor.selected()
        const [turn, own] = frames.length === 1 ? [editor.shown(frames[0]).rotation!, frames[0].rotation] : [0, 0]
        drag = { kind: 'resize', start: p, handle, box: bounds(frames.map(placed)), frames, snaps: snapsNow(), by: 1, turn, own }
        editor.beginGroup()
        return
      }
      const sel = editor.selection
      const hitId = pickAt(p, e.ctrlKey || e.metaKey ? 'deep' : 'click')
      const id = (!hitId || !sel.includes(hitId)) && maskedAt(p) || hitId
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
      const free = !(e.ctrlKey || e.metaKey)
      if (alt !== e.altKey && !drag) {
        alt = e.altKey
        redraw()
      }
      pointer = { offsetX: e.offsetX, offsetY: e.offsetY, ctrlKey: e.ctrlKey || e.metaKey }
      if (editor.preflight) {
        const ink = inkAt(p)
        if (ink !== editor.pointerInk) editor.set({ pointerInk: ink })
      }
      if ((editor.pen || editor.placing.length) && !drag) {
        cursor = p
        if (editor.placing.length) ({ p: cursor, guides: snapped } = placeAt(p, e.ctrlKey || e.metaKey))
        canvas.toggleAttribute('data-close', closes(p))
        redraw()
        return
      }
      if (!drag) return track()
      if (drag.kind === 'text') {
        const ed = editor.editing
        const id = inEdited(p)?.id ?? ed?.id
        if (ed && id) select(editor, ed.anchor, textAt(id, p))
      } else if (drag.kind === 'pan') {
        view.x += e.clientX - drag.last.x
        view.y += e.clientY - drag.last.y
        drag.last = { x: e.clientX, y: e.clientY }
        redraw()
      } else if (drag.kind === 'swap') {
        drag.at = p
        redraw()
      } else if (drag.kind === 'marquee') {
        drag.end = p
        const m = rect(drag.start, p)
        const inside = editor.spread
          .flatMap((page) => page.children.map((n) => ({ ...n, x: n.x + page.x })))
          .filter((n) => !n.hidden && !n.locked)
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
        if (!drag.id) draw(drag)
        const s = snapPoint(p, drag.snaps, !free || e.shiftKey)
        snapped = s.guides
        let dx = s.p.x - drag.start.x
        let dy = s.p.y - drag.start.y
        if (drag.tool === 'line') {
          if (e.shiftKey) [dx, dy] = snap45(dx, dy)
          const [x, y] = [drag.start.x - drag.dx, drag.start.y]
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
        editor.apply({ type: 'setFrame', id: drag.id, ...r, x: r.x - drag.dx })
      } else if (drag.kind === 'move') {
        let dx = p.x - drag.start.x
        let dy = p.y - drag.start.y
        if (!drag.active) {
          if (Math.hypot(dx, dy) * view.zoom <= DRAG) return
          drag.active = true
          editor.beginGroup()
          if (e.altKey) {
            editor.duplicate(drag.frames.map((n) => n.id), false)
            drag.frames = editor.selected()
          }
          drag.box = bounds(drag.frames.map(covered))
          drag.snaps = snapsNow()
        }
        if (drag.flow) {
          const local = { x: p.x - editor.dx(drag.flow.id), y: p.y }
          drag.to = insertion(drag.flow, drag.frames.map((n) => n.id), local)
          redraw()
          return
        }
        const lock = e.shiftKey ? (Math.abs(dx) > Math.abs(dy) ? 'y' : 'x') : undefined
        if (lock === 'y') dy = 0
        if (lock === 'x') dx = 0
        const { lines, others } = drag.snaps!
        const b = drag.box!
        const xs = (d: number) => (lock === 'x' ? [] : [b.x + d, b.x + d + b.w / 2, b.x + d + b.w])
        const ys = (d: number) => (lock === 'y' ? [] : [b.y + d, b.y + d + b.h / 2, b.y + d + b.h])
        if (free) {
          const tolerance = SNAP / view.zoom
          const by = (axis: 'x' | 'y', values: number[]) => {
            const line = snap(values, lines[axis], tolerance)
            if (!values.length) return line
            const gap = snap([values[0]], spacings({ ...b, x: b.x + dx, y: b.y + dy }, others, axis).map((at) => ({ at, from: 0, to: 0 })), tolerance)
            return gap && (!line || Math.abs(gap) < Math.abs(line)) ? gap : line
          }
          dx += by('x', xs(dx))
          dy += by('y', ys(dy))
        }
        const moved = { ...b, x: b.x + dx, y: b.y + dy }
        snapped = free ? guides(moved, lines, xs(dx), ys(dy)) : []
        const all = [...nearest(moved, others).flatMap((o) => measure(moved, o)), ...equals(moved, others, 'x'), ...equals(moved, others, 'y')]
        measures = [...new Map(all.map((m) => [`${m.x1} ${m.y1} ${m.x2} ${m.y2}`, m])).values()]
        for (const n of drag.frames) editor.apply({ type: 'setFrame', id: n.id, x: n.x + dx, y: n.y + dy, w: n.w, h: n.h })
      } else if (drag.kind === 'knot') {
        editor.setKnots(shift(drag.cs, drag.at, drag.part, p.x - drag.start.x, p.y - drag.start.y), drag.at)
      } else if (drag.kind === 'rotate') {
        let by = angle(drag.c, p) - drag.from
        if (e.shiftKey) by = Math.round(by / 15) * 15
        editor.turn(drag.nodes, by, drag.c)
        canvas.style.cursor = cursorOf(drag.handle, drag.turn + by)
      } else if (drag.kind === 'radius') {
        const { box: b, index: i } = drag
        const [sx, sy] = [i === 1 || i === 2 ? -1 : 1, i < 2 ? 1 : -1]
        const d = spin({ x: p.x - drag.start.x, y: p.y - drag.start.y }, -drag.turn)
        const r = Math.max(0, Math.min(drag.radii[i] + (d.x * sx + d.y * sy) / 2, b.w / 2, b.h / 2))
        const corners = drag.radii.map((v, j) => (j === i ? r : v))
        editor.apply({ type: 'set', id: drag.id, ...(e.ctrlKey || e.metaKey ? { corners } : { radius: r, corners: [] }) })
      } else if (drag.kind === 'end') {
        const fixed = drag.ends[1 - drag.index]
        const from = drag.ends[drag.index]
        let dx = from.x + p.x - drag.start.x - fixed.x
        let dy = from.y + p.y - drag.start.y - fixed.y
        if (e.shiftKey) [dx, dy] = snap45(dx, dy)
        const moved = { x: fixed.x + dx, y: fixed.y + dy }
        const [a, b] = drag.index ? [fixed, moved] : [moved, fixed]
        const off = editor.dx(drag.id)
        editor.apply({ type: 'setPath', id: drag.id, path: [0, a.x - off, a.y, 1, b.x - off, b.y] })
      } else if (drag.kind === 'resize') {
        const d = spin({ x: p.x - drag.start.x, y: p.y - drag.start.y }, -drag.turn)
        const { box: b, handle: h, snaps, own } = drag
        const edge = (side: string, at: number) => (h.includes(side) ? [at] : [])
        if (free && !drag.turn) {
          d.x += snap([...edge('w', b.x + d.x), ...edge('e', b.x + b.w + d.x)], snaps.lines.x, SNAP / view.zoom)
          d.y += snap([...edge('n', b.y + d.y), ...edge('s', b.y + b.h + d.y)], snaps.lines.y, SNAP / view.zoom)
        }
        const boxes = resized(b, h, d, e, drag.frames.map(placed)).map((f) => {
          const c = { x: f.x + f.w / 2 - b.x - b.w / 2, y: f.y + f.h / 2 - b.y - b.h / 2 }
          const m = spin(c, own)
          return { ...f, x: f.x + m.x - c.x, y: f.y + m.y - c.y }
        })
        const r = bounds(boxes)
        snapped = free && !drag.turn ? guides(r, snaps.lines, [...edge('w', r.x), ...edge('e', r.x + r.w)], [...edge('n', r.y), ...edge('s', r.y + r.h)]) : []
        const by = h === 'e' || h === 'w' ? r.w / b.w : r.h / b.h
        const scale = (e.ctrlKey || e.metaKey) && by > 0 ? by / drag.by : 1
        drag.by *= scale
        drag.frames.forEach((n, i) => {
          const f = boxes[i]
          if (scale !== 1 && n.kind === 'text') editor.apply({ type: 'scaleText', id: n.id, by: scale })
          editor.apply({ type: 'setFrame', id: n.id, ...f, x: f.x - editor.dx(n.id), ignoreConstraints: e.ctrlKey || e.metaKey })
          const t = editor.nodes.get(n.id)?.node
          if ((h === 'e' || h === 'w') && t?.kind === 'text' && t.overset && !t.next && t.sizing.vertical === 'fixed') {
            editor.apply({ type: 'set', id: n.id, sizing: { ...t.sizing, vertical: 'hug' } })
          }
        })
      }
    }
    const onPointerUp = (e: PointerEvent) => {
      if (drag?.kind === 'draw' && drag.thread) {
        const from = editor.lookup(drag.thread)?.node
        if (!drag.moved && from) editor.apply({ type: 'setFrame', id: drag.id, x: drag.start.x - drag.dx, y: drag.start.y, w: from.w, h: from.h })
        const ok = editor.engine.canThread(drag.thread, drag.id)
        editor.apply({ type: 'thread', from: drag.thread, to: drag.id })
        if (ok) editor.set({ threading: null, selection: [drag.id] })
        else {
          editor.apply({ type: 'delete', ids: [drag.id] })
          editor.set({ selection: [] })
        }
      } else if (drag?.kind === 'draw' && !drag.id) {
        editor.setTool('move')
        editor.set({ selection: [] })
      } else if (drag?.kind === 'draw') {
        if (drag.tool !== 'text' && !drag.moved) {
          const [w, h] = DEFAULT_SIZE[drag.tool]
          editor.apply({ type: 'setFrame', id: drag.id, x: drag.start.x - drag.dx, y: drag.start.y, w: w * MM, h: h * MM })
        }
        editor.setTool('move')
        const n = editor.nodes.get(drag.id)?.node
        if (n?.kind === 'text') editor.set({ editing: { id: n.id, anchor: 0, focus: editor.storyOf(n).text.length } })
      }
      if (drag?.kind === 'swap') {
        const { id, at } = drag
        const to = swaps().find((s) => s.id !== id && at.x >= s.box.x && at.x <= s.box.x + s.box.w && at.y >= s.box.y && at.y <= s.box.y + s.box.h)
        if (to) swap(id, to.id)
      }
      if (drag?.kind === 'move' && drag.flow && drag.to) {
        editor.apply({ type: 'move', ids: drag.frames.map((n) => n.id), parent: drag.flow.id, index: drag.to.index })
      } else if (drag?.kind === 'move' && drag.active) {
        // A layer dragged across the spine goes to the page its centre is on.
        const moves = new Map<Page, string[]>()
        for (const { id } of drag.frames) {
          const entry = editor.nodes.get(id)
          if (!entry || entry.parent) continue
          const n = placed(entry.node)
          const to = pageAt({ x: n.x + n.w / 2, y: n.y + n.h / 2 })
          if (to.id !== entry.page?.id) moves.set(to, [...(moves.get(to) ?? []), id])
        }
        for (const [to, ids] of moves) editor.apply({ type: 'move', ids, parent: to.id, index: to.children.length })
      }
      if ((drag?.kind === 'draw' && drag.id) || drag?.kind === 'resize' || drag?.kind === 'end' || drag?.kind === 'radius' || drag?.kind === 'rotate' || drag?.kind === 'knot' || (drag?.kind === 'move' && drag.active)) {
        editor.endGroup()
      }
      if (drag?.kind === 'move' && !drag.active && !e.shiftKey) {
        const id = pickAt(toDoc(e), e.ctrlKey || e.metaKey ? 'deep' : 'click')
        if (id && editor.selection.length > 1) editor.set({ selection: [id] })
      }
      drag = null
      snapped = []
      measures = []
      editor.dragging = false
      delete canvas.dataset.panning
      track()
      redraw()
    }
    const onDoubleClick = (e: MouseEvent) => {
      if (editor.tool !== 'move' || editor.vector) return
      const port = portUnder(e)
      if (port) {
        // Double-clicking a port breaks the thread there.
        const id = port.side === 'out' ? port.node.id : port.node.prev
        if (id && (port.side === 'in' || port.node.next)) editor.apply({ type: 'unthread', id })
        editor.set({ threading: null })
        return
      }
      const [one] = editor.selected()
      const handle = editor.selection.length === 1 && one.kind === 'text' ? handleUnder(e) : undefined
      if (handle && /^[nsew]{1,2}$/.test(handle)) {
        editor.resize([one], handle.length === 2 ? 'autoFit' : handle === 'n' || handle === 's' ? 'autoHeight' : 'autoWidth')
        return
      }
      const p = toDoc(e)
      const edited = inEdited(p)
      if (edited) {
        const [a, b] = wordAt(editor.storyOf(edited).text, textAt(edited.id, p))
        if (editor.editing!.anchor === editor.editing!.focus) select(editor, a, b)
        return
      }
      const id = pickAt(p, 'double')
      const n = id && editor.nodes.get(id)?.node
      if (n && n.kind === 'text' && editor.selection.includes(id)) {
        const i = textAt(n.id, p)
        editor.set({ editing: { id: n.id, anchor: i, focus: i } })
      } else if (n && n.kind === 'shape' && editor.selection.includes(id)) editor.editPath(id)
      else if (id) editor.set({ selection: [id] })
    }
    /** A triple click in the edited text selects all of it. */
    const onClick = (e: MouseEvent) => {
      const edited = e.detail >= 3 && editor.tool === 'move' ? inEdited(toDoc(e)) : undefined
      if (edited) select(editor, 0, editor.storyOf(edited).text.length)
    }
    const onLeave = () => {
      pointer = undefined
      editor.pointer = undefined
      hover = undefined
      cursor = undefined
      if (editor.pointerInk !== null) editor.set({ pointerInk: null })
      redraw()
    }
    const onKey = (e: KeyboardEvent) => {
      if (isTyping(e)) return
      if (e.key === 'Alt') {
        alt = e.type === 'keydown'
        e.preventDefault()
        redraw()
      }
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
      settle()
      const cx = canvas.clientWidth / 2
      const cy = canvas.clientHeight / 2
      const mod = e.ctrlKey || e.metaKey
      const sel = editor.selection.length ? bounds(editor.selected().map(placed)) : undefined
      if (e.shiftKey && e.code === 'Digit1') glide(fitView(editor.sheets, canvas.clientWidth, canvas.clientHeight))
      else if (e.shiftKey && e.code === 'Digit2' && sel) {
        const zoom = Math.min(256, (canvas.clientWidth - 240) / Math.max(sel.w, 1), (canvas.clientHeight - 240) / Math.max(sel.h, 1))
        glide({ x: cx - (sel.x + sel.w / 2) * zoom, y: cy - (sel.y + sel.h / 2) * zoom, zoom })
      } else if ((e.shiftKey || mod) && e.code === 'Digit0') glide(around(cx, cy, PX_PER_PT))
      else if (mod && (e.key === '=' || e.key === '+')) zoomAt(cx, cy, view.zoom * 1.25)
      else if (mod && e.key === '-') zoomAt(cx, cy, view.zoom / 1.25)
      else return
      track()
      e.preventDefault()
    }

    let side = editor.side
    const offSettings = subscribeSettings(redraw)
    const unsubscribe = editor.subscribe(() => {
      if (editor.side !== side) {
        side = editor.side
        fit(editor.sheets.slice(side === 'left' ? 0 : -1).slice(0, 1))
      }
      if (spreadKey() !== shown) {
        shown = spreadKey()
        fit()
      }
      wake()
      canvas.dataset.tool = editor.tool
      canvas.toggleAttribute('data-threading', editor.threading !== null)
      track()
      redraw()
    })
    canvas.parentElement!.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('pointercancel', onPointerUp)
    canvas.addEventListener('pointerleave', onLeave)
    canvas.addEventListener('dblclick', onDoubleClick)
    canvas.addEventListener('click', onClick)
    window.addEventListener('keydown', onKey)
    window.addEventListener('keyup', onKey)
    const scheme = matchMedia('(prefers-color-scheme: light)')
    scheme.addEventListener('change', redraw)
    return () => {
      scheme.removeEventListener('change', redraw)
      cancelAnimationFrame(frame)
      cancelAnimationFrame(anim)
      clearInterval(blink)
      unsubscribe()
      offSettings()
      resize.disconnect()
      canvas.parentElement!.removeEventListener('wheel', onWheel)
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
      gl.delete()
    }
  }, [ck, editor])

  return (
    <div className="stage">
      <div className="ruler-corner" />
      <canvas ref={rulerX} className="ruler ruler-x" aria-hidden="true" />
      <canvas ref={rulerY} className="ruler ruler-y" aria-hidden="true" />
      <div className="view">
        <canvas ref={ref} className="canvas" aria-label="Page canvas" tabIndex={-1} data-tool="move" />
        <svg ref={marks} className="marks" aria-hidden="true" />
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
        <output className="sr-only" aria-label="Zoom">
          {Math.round((zoom / PX_PER_PT) * 100)}%
        </output>
        <div ref={quick} className="quick" role="toolbar" aria-label="Quick edit">
          <Quick editor={editor} />
        </div>
        <Switcher editor={editor} />
      </div>
    </div>
  )
}
