import type { Node } from './model'

export type Entry = { node: Node; parent?: Node }

export function index(nodes: Node[], parent?: Node, out = new Map<string, Entry>()) {
  for (const node of nodes) {
    out.set(node.id, { node, parent })
    if ('children' in node) index(node.children, node, out)
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
  let i = 0
  while (i < path.length - 1 && all.get(path[i])?.node.kind === 'frame') i++
  return path[i]
}
