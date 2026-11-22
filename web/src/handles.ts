import type { Point } from './editor'
import { HANDLE, type Box, type View } from './renderer'

const EDGE = 4
const PORT_INSET = 24
const PORT = 10
const TURN = 16
const RADIUS = 12

const screen = (view: View, p: Point) => ({ x: view.x + p.x * view.zoom, y: view.y + p.y * view.zoom })

export function spin(p: Point, degrees: number, c: Point = { x: 0, y: 0 }): Point {
  const [s, k] = [Math.sin((degrees * Math.PI) / 180), Math.cos((degrees * Math.PI) / 180)]
  const [x, y] = [p.x - c.x, p.y - c.y]
  return { x: c.x + k * x + s * y, y: c.y - s * x + k * y }
}

export function upright(b: Box): Box {
  const c = { x: b.x + b.w / 2, y: b.y + b.h / 2 }
  const ps = ([[0, 0], [b.w, 0], [b.w, b.h], [0, b.h]] as const).map(([u, v]) => spin({ x: b.x + u, y: b.y + v }, b.rotation ?? 0, c))
  const [xs, ys] = [ps.map((p) => p.x), ps.map((p) => p.y)]
  return rect({ x: Math.min(...xs), y: Math.min(...ys) }, { x: Math.max(...xs), y: Math.max(...ys) })
}

export function handleAt(view: View, x: number, y: number, { box, line, radii, gaps }: { box?: Box; line?: [Point, Point]; radii?: Point[]; gaps?: Gap[] }) {
  const g = gaps?.findIndex((p) => Math.hypot(p.x - x, p.y - y) <= HANDLE) ?? -1
  const gap = gaps?.[g]
  if (gap) return `gap${gap.axis}${g}`
  if (box?.rotation) {
    const c = screen(view, { x: box.x + box.w / 2, y: box.y + box.h / 2 })
    ;({ x, y } = spin({ x, y }, -box.rotation, c))
  }
  const i = radii?.findIndex((p) => Math.hypot(p.x - x, p.y - y) <= HANDLE + 1) ?? -1
  if (i >= 0) return `radius${i}`
  if (line) {
    const [a, b] = [screen(view, line[0]), screen(view, line[1])]
    const i = [a, b].findIndex((p) => Math.hypot(p.x - x, p.y - y) <= HANDLE / 2 + 1)
    if (i >= 0) return `end${i}`
    const beyond = ([e, o]: [Point, Point]) => Math.hypot(e.x - x, e.y - y) <= TURN && (x - e.x) * (e.x - o.x) + (y - e.y) * (e.y - o.y) > 0
    return beyond([a, b]) ? 'rotatenw' : beyond([b, a]) ? 'rotatese' : undefined
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
  if (!within(x, l, r) || !within(y, t, b)) {
    for (const [cy, v] of [[t, 'n'], [b, 's']] as const) {
      for (const [cx, h] of [[l, 'w'], [r, 'e']] as const) if (Math.hypot(x - cx, y - cy) <= TURN) return `rotate${v}${h}`
    }
    return undefined
  }
  if (box.h && near(y, t, EDGE)) return 'n'
  if (box.h && near(y, b, EDGE)) return 's'
  if (box.w && near(x, l, EDGE)) return 'w'
  if (box.w && near(x, r, EDGE)) return 'e'
  return undefined
}

export type Gap = Point & { axis: 'x' | 'y' }

export function gapHandles(view: View, axis: 'x' | 'y', boxes: Box[]): Gap[] {
  const [x, w, y, h] = axis === 'x' ? (['x', 'w', 'y', 'h'] as const) : (['y', 'h', 'x', 'w'] as const)
  return boxes.flatMap((b, i) => {
    const a = boxes[i - 1]
    if (!a) return []
    const p = { [x]: (a[x] + a[w] + b[x]) / 2, [y]: (Math.max(a[y], b[y]) + Math.min(a[y] + a[h], b[y] + b[h])) / 2 } as Point
    return [{ ...screen(view, p), axis }]
  })
}

export function radiusHandles(view: View, box: Box, radii: number[]) {
  const [w, h] = [box.w * view.zoom, box.h * view.zoom]
  if (Math.min(w, h) < 4 * RADIUS) return []
  const { x: l, y: t } = screen(view, box)
  return radii.map((r, i) => {
    const d = Math.min(Math.max(r * view.zoom, RADIUS), w / 2, h / 2)
    return { x: i === 1 || i === 2 ? l + w - d : l + d, y: i < 2 ? t + d : t + h - d }
  })
}

export function portsOf(view: View, n: Box) {
  const inset = Math.min(PORT_INSET / view.zoom, n.h / 4)
  const c = { x: n.x + n.w / 2, y: n.y + n.h / 2 }
  const at = (x: number, y: number) => screen(view, spin({ x, y }, n.rotation ?? 0, c))
  return { in: at(n.x, n.y + inset), out: at(n.x + n.w, n.y + n.h - inset) }
}

export function portAt(ports: { in?: Point; out: Point }, x: number, y: number) {
  const near = (q?: Point) => !!q && Math.abs(q.x - x) <= PORT / 2 + 1 && Math.abs(q.y - y) <= PORT / 2 + 1
  return near(ports.out) ? ('out' as const) : near(ports.in) ? ('in' as const) : undefined
}

export function rect(a: Point, b: Point): Box {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) }
}

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
    const sw = Math.abs((r - l) / box.w)
    const sh = Math.abs((b - t) / box.h)
    const s = !/[ns]/.test(handle) ? sw : !/[ew]/.test(handle) ? sh : Math.max(sw, sh)
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
