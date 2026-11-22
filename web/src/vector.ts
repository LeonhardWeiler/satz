import type { Point } from './editor'

const [MOVE, LINE, CUBIC, CLOSE] = [0, 1, 4, 5]

export type Knot = { x: number; y: number; ix: number; iy: number; ox: number; oy: number }
export type Contour = { knots: Knot[]; closed: boolean }
export type At = [c: number, i: number]

const knot = (x: number, y: number): Knot => ({ x, y, ix: x, iy: y, ox: x, oy: y })
const lerp = (a: Point, b: Point, t: number) => ({ x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t })
const isLine = (a: Knot, b: Knot) => a.ox === a.x && a.oy === a.y && b.ix === b.x && b.iy === b.y

export function contours(path: ArrayLike<number>): Contour[] {
  const out: Contour[] = []
  let knots: Knot[] = []
  for (let i = 0; i < path.length; ) {
    const v = path[i]
    const p = (k: number) => path[i + k] ?? 0
    if (v === MOVE) {
      knots = [knot(p(1), p(2))]
      out.push({ knots, closed: false })
      i += 3
    } else if (v === LINE) {
      knots.push(knot(p(1), p(2)))
      i += 3
    } else if (v === CUBIC) {
      Object.assign(knots.at(-1) ?? {}, { ox: p(1), oy: p(2) })
      knots.push({ ...knot(p(5), p(6)), ix: p(3), iy: p(4) })
      i += 7
    } else {
      const contour = out.at(-1)
      if (contour) contour.closed = true
      const [first, last] = [knots[0], knots.at(-1)]
      if (first && last && knots.length > 1 && Math.hypot(first.x - last.x, first.y - last.y) < 1e-3) {
        Object.assign(first, { ix: last.ix, iy: last.iy })
        knots.pop()
      }
      i += 1
    }
  }
  return out
}

export function toPath(cs: Contour[]): number[] {
  const path: number[] = []
  const segment = (a: Knot, b: Knot) => {
    if (isLine(a, b)) path.push(LINE, b.x, b.y)
    else path.push(CUBIC, a.ox, a.oy, b.ix, b.iy, b.x, b.y)
  }
  for (const { knots, closed } of cs) {
    const [first, ...rest] = knots
    if (!first) continue
    path.push(MOVE, first.x, first.y)
    let last = first
    for (const k of rest) {
      segment(last, k)
      last = k
    }
    if (closed) {
      segment(last, first)
      path.push(CLOSE)
    }
  }
  return path
}

function segments(cs: Contour[]) {
  return cs.flatMap(({ knots, closed }, c) =>
    knots.flatMap((a, i) => {
      const b = knots[(i + 1) % knots.length]
      return b && (i + 1 < knots.length || (closed && knots.length > 1)) ? [{ at: [c, i] as At, a, b }] : []
    }),
  )
}

function bezier(a: Knot, b: Knot, t: number) {
  const [p0, p1, p2, p3] = [a, { x: a.ox, y: a.oy }, { x: b.ix, y: b.iy }, b]
  const [p01, p12, p23] = [lerp(p0, p1, t), lerp(p1, p2, t), lerp(p2, p3, t)]
  const [p012, p123] = [lerp(p01, p12, t), lerp(p12, p23, t)]
  return { p01, p23, p012, p123, p: lerp(p012, p123, t) }
}

export function nearest(cs: Contour[], p: Point) {
  let best = { at: [0, 0] as At, t: 0, d: Infinity }
  for (const { at, a, b } of segments(cs)) {
    for (let k = 1; k < 64; k++) {
      const q = bezier(a, b, k / 64).p
      const d = Math.hypot(q.x - p.x, q.y - p.y)
      if (d < best.d) best = { at, t: k / 64, d }
    }
  }
  return best
}

export function split(cs: Contour[], [c, i]: At, t: number): Contour[] {
  const out = structuredClone(cs)
  const knots = out[c]?.knots ?? []
  const [a, b] = [knots[i], knots[(i + 1) % knots.length]]
  if (!a || !b) return out
  const { p01, p23, p012, p123, p } = bezier(a, b, t)
  const line = isLine(a, b)
  if (!line) {
    Object.assign(a, { ox: p01.x, oy: p01.y })
    Object.assign(b, { ix: p23.x, iy: p23.y })
  }
  knots.splice(i + 1, 0, line ? knot(p.x, p.y) : { x: p.x, y: p.y, ix: p012.x, iy: p012.y, ox: p123.x, oy: p123.y })
  return out
}

export function remove(cs: Contour[], at: At[]): Contour[] {
  return cs
    .map((k, c) => ({ ...k, knots: k.knots.filter((_, i) => !at.some((a) => a[0] === c && a[1] === i)) }))
    .filter((k) => k.knots.length > 1)
}

export const curved = (k: Knot) => k.ix !== k.x || k.iy !== k.y || k.ox !== k.x || k.oy !== k.y

export function smooth(cs: Contour[], [c, i]: At, on: boolean): Contour[] {
  const out = structuredClone(cs)
  const { knots = [], closed = false } = out[c] ?? {}
  const n = knots.length
  const [k, prev, next] = [i, closed ? (i + n - 1) % n : Math.max(i - 1, 0), closed ? (i + 1) % n : Math.min(i + 1, n - 1)].map((j) => knots[j])
  if (!k || !prev || !next) return out
  const [dx, dy] = [next.x - prev.x, next.y - prev.y]
  const len = (Math.hypot(dx, dy) || 1) * 3
  const [a, b] = on ? [Math.hypot(k.x - prev.x, k.y - prev.y) / len, Math.hypot(next.x - k.x, next.y - k.y) / len] : [0, 0]
  Object.assign(k, { ix: k.x - dx * a, iy: k.y - dy * a, ox: k.x + dx * b, oy: k.y + dy * b })
  return out
}

export function shift(cs: Contour[], at: At[], part: 'point' | 'in' | 'out', dx: number, dy: number): Contour[] {
  const out = structuredClone(cs)
  for (const [c, i] of at) {
    const k = out[c]?.knots[i]
    if (!k) continue
    if (part !== 'out') Object.assign(k, { ix: k.ix + dx, iy: k.iy + dy })
    if (part !== 'in') Object.assign(k, { ox: k.ox + dx, oy: k.oy + dy })
    if (part === 'point') Object.assign(k, { x: k.x + dx, y: k.y + dy })
  }
  return out
}
