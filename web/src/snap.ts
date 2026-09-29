import type { Node } from './model'
import type { Box, Sheet } from './renderer'

/** A line to snap to at `at` on its axis, spanning `from` to `to` on the other. */
export type Line = { at: number; from: number; to: number }
export type Lines = { x: Line[]; y: Line[] }
/** A guide along x = `at` or y = `at`, from `from` to `to` on the other axis. */
export type Guide = { axis: 'x' | 'y'; at: number; from: number; to: number }
/** A distance from (x1, y1) to (x2, y2); a dashed one only leads to what is measured. */
export type Measure = { x1: number; y1: number; x2: number; y2: number; length: number; dashed?: boolean }

const EPSILON = 1e-6

/** The columns and rows of the layout grids of `s` as spans on their axis in the space of the spread; the lines of a grid span nothing. */
export function gridSpans(s: Sheet) {
  const x: [number, number][] = []
  const y: [number, number][] = []
  for (const g of s.grids ?? []) {
    if (g.kind === 'grid') {
      for (let at = g.size; at < s.width - EPSILON; at += g.size) x.push([s.x + at, s.x + at])
      for (let at = g.size; at < s.height - EPSILON; at += g.size) y.push([at, at])
      continue
    }
    const [length, start, out] = g.kind === 'columns' ? [s.width, s.x, x] : [s.height, 0, y]
    const size = (length - 2 * g.margin - g.gutter * (g.count - 1)) / g.count
    for (let i = 0; i < g.count; i++) {
      const at = start + g.margin + i * (size + g.gutter)
      out.push([at, at + size])
    }
  }
  return { x, y }
}

/**
 * What layers snap to on a spread: edges and centres of the pages and of `nodes`, of text
 * frames also their insets, column edges and baseline grid unless turned, and with `grids` the layout grids.
 * Everything is in the space of the spread.
 */
export function targets(sheets: Sheet[], nodes: Node[], grids = false): Lines {
  const x: Line[] = []
  const y: Line[] = []
  const box = (b: Box) => {
    for (const at of [b.x, b.x + b.w / 2, b.x + b.w]) x.push({ at, from: b.y, to: b.y + b.h })
    for (const at of [b.y, b.y + b.h / 2, b.y + b.h]) y.push({ at, from: b.x, to: b.x + b.w })
  }
  for (const s of sheets) {
    box({ x: s.x, y: 0, w: s.width, h: s.height })
    if (!grids) continue
    const spans = gridSpans(s)
    for (const at of spans.x.flat()) x.push({ at, from: 0, to: s.height })
    for (const at of spans.y.flat()) y.push({ at, from: s.x, to: s.x + s.width })
  }
  for (const n of nodes) {
    box(n)
    if (n.kind !== 'text' || n.rotation) continue
    const [l, r, t, b] = [n.x + n.insetLeft, n.x + n.w - n.insetRight, n.y + n.insetTop, n.y + n.h - n.insetBottom]
    const column = (r - l - n.gutter * (n.columns - 1)) / n.columns
    for (let c = 0; c < n.columns; c++) {
      const at = l + c * (column + n.gutter)
      x.push({ at, from: n.y, to: n.y + n.h }, { at: at + column, from: n.y, to: n.y + n.h })
    }
    y.push({ at: t, from: n.x, to: n.x + n.w }, { at: b, from: n.x, to: n.x + n.w })
    if (n.baselineGrid > 0) for (let at = t + n.baselineStart; at <= b + EPSILON; at += n.baselineGrid) y.push({ at, from: l, to: r })
  }
  return { x, y }
}

/** The offset that moves the nearest of `values` onto one of `lines` within `tolerance`, or 0. */
export function snap(values: number[], lines: Line[], tolerance: number) {
  let best = 0
  let away = tolerance + EPSILON
  for (const v of values) {
    for (const l of lines) {
      const d = l.at - v
      if (Math.abs(d) < away) [best, away] = [d, Math.abs(d)]
    }
  }
  return best
}

/** Guides through each of `xs` and `ys` that lies on one of `lines`, spanning the line and `box`. */
export function guides(box: Box, lines: Lines, xs: number[], ys: number[]): Guide[] {
  const out: Guide[] = []
  const along = (axis: 'x' | 'y', values: number[], list: Line[], from: number, to: number) => {
    for (const at of new Set(values)) {
      const on = list.filter((l) => Math.abs(l.at - at) < EPSILON)
      if (on.length) out.push({ axis, at, from: Math.min(from, ...on.map((l) => l.from)), to: Math.max(to, ...on.map((l) => l.to)) })
    }
  }
  along('x', xs, lines.x, box.y, box.y + box.h)
  along('y', ys, lines.y, box.x, box.x + box.w)
  return out
}

/** The `n` of `others` nearest to `box` that do not overlap it. */
export function nearest(box: Box, others: Box[], n = 2) {
  return others
    .map((o) => {
      const dx = Math.max(o.x - box.x - box.w, box.x - o.x - o.w, 0)
      const dy = Math.max(o.y - box.y - box.h, box.y - o.y - o.h, 0)
      return { o, d: Math.hypot(dx, dy) }
    })
    .filter((e) => e.d > 0)
    .sort((a, b) => a.d - b.d)
    .slice(0, n)
    .map((e) => e.o)
}

/**
 * The distances between `a` and `b`: when one is inside the other, from each side of the inner
 * one to the nearest of `others` on that side or else to the outer one; else across the gap on
 * each axis they are apart on.
 */
export function measure(a: Box, b: Box, others: Box[] = []): Measure[] {
  const out: Measure[] = []
  const h = (x1: number, x2: number, y: number) => {
    if (x2 - x1 > EPSILON) out.push({ x1, y1: y, x2, y2: y, length: x2 - x1 })
  }
  const v = (y1: number, y2: number, x: number) => {
    if (y2 - y1 > EPSILON) out.push({ x1: x, y1, x2: x, y2, length: y2 - y1 })
  }
  const inside = (p: Box, q: Box) =>
    p.x >= q.x - EPSILON && p.y >= q.y - EPSILON && p.x + p.w <= q.x + q.w + EPSILON && p.y + p.h <= q.y + q.h + EPSILON
  if (inside(a, b) || inside(b, a)) {
    const [i, u] = inside(a, b) ? [a, b] : [b, a]
    const [ir, ib, ur, ub] = [i.x + i.w, i.y + i.h, u.x + u.w, u.y + u.h]
    const row = others.filter((o) => o.y < ib && o.y + o.h > i.y)
    const column = others.filter((o) => o.x < ir && o.x + o.w > i.x)
    const y = (o?: Box) => (o ? (Math.max(i.y, o.y) + Math.min(ib, o.y + o.h)) / 2 : i.y + i.h / 2)
    const x = (o?: Box) => (o ? (Math.max(i.x, o.x) + Math.min(ir, o.x + o.w)) / 2 : i.x + i.w / 2)
    const l = row.filter((o) => o.x + o.w <= i.x + EPSILON && o.x + o.w > u.x).sort((p, q) => q.x + q.w - p.x - p.w)[0]
    const r = row.filter((o) => o.x >= ir - EPSILON && o.x < ur).sort((p, q) => p.x - q.x)[0]
    const t = column.filter((o) => o.y + o.h <= i.y + EPSILON && o.y + o.h > u.y).sort((p, q) => q.y + q.h - p.y - p.h)[0]
    const d = column.filter((o) => o.y >= ib - EPSILON && o.y < ub).sort((p, q) => p.y - q.y)[0]
    h(l ? l.x + l.w : u.x, i.x, y(l))
    h(ir, r ? r.x : ur, y(r))
    v(t ? t.y + t.h : u.y, i.y, x(t))
    v(ib, d ? d.y : ub, x(d))
    return out
  }
  const oy = [Math.max(a.y, b.y), Math.min(a.y + a.h, b.y + b.h)]
  const ox = [Math.max(a.x, b.x), Math.min(a.x + a.w, b.x + b.w)]
  if (b.x >= a.x + a.w || a.x >= b.x + b.w) {
    const right = b.x >= a.x + a.w
    const [l, r] = right ? [a.x + a.w, b.x] : [b.x + b.w, a.x]
    const y = oy[0] < oy[1] ? (oy[0] + oy[1]) / 2 : a.y + a.h / 2
    h(l, r, y)
    if (oy[0] >= oy[1]) {
      const x = right ? b.x : b.x + b.w
      const to = b.y > y ? b.y : b.y + b.h
      out.push({ x1: x, y1: y, x2: x, y2: to, length: Math.abs(to - y), dashed: true })
    }
  }
  if (b.y >= a.y + a.h || a.y >= b.y + b.h) {
    const below = b.y >= a.y + a.h
    const [t, u] = below ? [a.y + a.h, b.y] : [b.y + b.h, a.y]
    const x = ox[0] < ox[1] ? (ox[0] + ox[1]) / 2 : a.x + a.w / 2
    v(t, u, x)
    if (ox[0] >= ox[1]) {
      const y = below ? b.y : b.y + b.h
      const to = b.x > x ? b.x : b.x + b.w
      out.push({ x1: x, y1: y, x2: to, y2: y, length: Math.abs(to - x), dashed: true })
    }
  }
  return out
}
