import type { Point } from './editor'
import { HANDLE, type Box, type View } from './renderer'

/** Distance in px from an edge of the selection box within which the edge is grabbed. */
const EDGE = 4
/** Distance in px of a text frame's in- and out-port from its top and bottom corner. */
const PORT_INSET = 16
const PORT = 10

const screen = (view: View, p: Point) => ({ x: view.x + p.x * view.zoom, y: view.y + p.y * view.zoom })

/**
 * What the screen point (x, y) grabs of the selection handles: `end0` or `end1` of a
 * line, a corner such as `nw` or an edge such as `e` of the box, or nothing.
 */
export function handleAt(view: View, x: number, y: number, { box, line }: { box?: Box; line?: [Point, Point] }) {
  if (line) {
    const i = line.findIndex((p) => Math.hypot(screen(view, p).x - x, screen(view, p).y - y) <= HANDLE / 2 + 1)
    return i < 0 ? undefined : `end${i}`
  }
  if (!box) return undefined
  const { x: l, y: t } = screen(view, box)
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

/**
 * The in- and out-port in screen space of a text frame placed on its spread, as in
 * InDesign: on its left edge below the top and its right edge above the bottom.
 */
export function portsOf(view: View, n: Box) {
  const inset = Math.min(PORT_INSET, (n.h * view.zoom) / 2)
  const tl = screen(view, n)
  const br = screen(view, { x: n.x + n.w, y: n.y + n.h })
  return { in: { x: tl.x, y: tl.y + inset }, out: { x: br.x, y: br.y - inset } }
}

/** The port of `ports` at the screen point (x, y). */
export function portAt(ports: { in: Point; out: Point }, x: number, y: number) {
  const near = (q: Point) => Math.abs(q.x - x) <= PORT / 2 + 1 && Math.abs(q.y - y) <= PORT / 2 + 1
  return near(ports.out) ? ('out' as const) : near(ports.in) ? ('in' as const) : undefined
}

export function rect(a: Point, b: Point): Box {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) }
}

/**
 * The boxes of `frames` in the selection box `box` after its `handle` was dragged by
 * `d`: Alt resizes about the centre, Shift keeps the proportions. Everything is in the
 * space of the spread.
 */
export function resized(box: Box, handle: string, d: Point, keys: { altKey: boolean; shiftKey: boolean }, frames: Box[]) {
  let l = box.x + (handle.includes('w') ? d.x : 0)
  let r = box.x + box.w + (handle.includes('e') ? d.x : 0)
  let t = box.y + (handle.includes('n') ? d.y : 0)
  let b = box.y + box.h + (handle.includes('s') ? d.y : 0)
  if (keys.altKey) {
    if (handle.includes('w')) r = box.x + box.w - d.x
    if (handle.includes('e')) l = box.x - d.x
    if (handle.includes('n')) b = box.y + box.h - d.y
    if (handle.includes('s')) t = box.y - d.y
  }
  if (keys.shiftKey && box.w && box.h) {
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
  return frames.map((n) => {
    const x = l + (n.x - box.x) * sx
    const y = t + (n.y - box.y) * sy
    return rect({ x, y }, { x: x + n.w * sx, y: y + n.h * sy })
  })
}
