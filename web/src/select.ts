import type { Container, Node, Page } from './model'
import type { Point } from './editor'
import { spin } from './handles'
import type { Box } from './renderer'

export type Entry = { node: Node; parent?: Container; page?: Page }

export function index(nodes: Node[], parent?: Container, out = new Map<string, Entry>(), page?: Page) {
  for (const node of nodes) {
    out.set(node.id, { node, parent, page })
    if ('children' in node) index(node.children, node, out, page)
  }
  return out
}

export function pick(nodes: Node[], path: string[], selection: string[], mode: 'click' | 'double' | 'deep') {
  if (mode === 'deep') return path.at(-1)
  if (mode === 'double') {
    const i = path.findIndex((id) => selection.includes(id))
    if (i >= 0) return path[Math.min(i + 1, path.length - 1)]
  }
  const all = index(nodes)
  const parents = new Set(selection.map((id) => all.get(id)?.parent?.id))
  for (let i = path.length - 1; i > 0; i--) if (parents.has(path[i - 1])) return path[i]
  return path.find((id, i) => i === path.length - 1 || all.get(id)?.node.kind !== 'frame')
}

export function span(list: string[], selection: string[], id: string) {
  const at = selection.map((s) => list.indexOf(s)).filter((i) => i >= 0)
  const c = list.indexOf(id)
  if (!at.length || c < 0) return null
  const [a, b] = [Math.min(...at), Math.max(...at)]
  const range = c < a ? list.slice(c, a) : list.slice(b + 1, c + 1)
  return [...selection.filter((s) => s !== id && (c < a || list.indexOf(s) <= c)), ...range.filter((s) => s !== id), id]
}

export function marqueed(nodes: Node[], from: Point, m: Box, shown: (n: Node) => Box): string[] {
  const live = (list: Node[]) => list.filter((n) => !n.hidden && !n.locked)
  const scope = (list: Node[]): Node[] => {
    const n = list.findLast((n) => 'children' in n && overlap(corners(shown(n)), corners({ ...from, w: 0, h: 0 })))
    return n && 'children' in n ? scope(live(n.children)) : list
  }
  const area = corners(m)
  const inside = (p: Point) => p.x >= m.x && p.x <= m.x + m.w && p.y >= m.y && p.y <= m.y + m.h
  const swept = (list: Node[]): string[] =>
    list.flatMap((n) => {
      const c = corners(shown(n))
      if (!overlap(c, area)) return []
      const within = n.kind === 'frame' && !c.every(inside) ? swept(live(n.children)) : []
      return within.length ? within : [n.id]
    })
  return swept(scope(live(nodes)))
}

const corners = (b: Box) =>
  ([[0, 0], [b.w, 0], [b.w, b.h], [0, b.h]] as const).map(([u, v]) => spin({ x: b.x + u, y: b.y + v }, b.rotation ?? 0, { x: b.x + b.w / 2, y: b.y + b.h / 2 }))

const overlap = (p: Point[], q: Point[]) =>
  [p, q].every((s) =>
    [0, 1].every((i) => {
      const a = { x: s[i]!.y - s[i + 1]!.y, y: s[i + 1]!.x - s[i]!.x }
      const [lo, hi] = [p, q].map((t) => t.map((c) => c.x * a.x + c.y * a.y))
      return (!a.x && !a.y) || (Math.min(...lo!) < Math.max(...hi!) && Math.min(...hi!) < Math.max(...lo!))
    }),
  )
