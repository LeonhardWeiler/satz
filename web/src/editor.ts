import { useSyncExternalStore } from 'react'
import type { Engine } from './engine/engine'
import type { Command, Node, Snapshot } from './model'
import { penPath, type Anchor } from './pen'
import { index, type Entry } from './select'

export type Shape = 'rect' | 'line' | 'arrow' | 'ellipse' | 'polygon' | 'star'
export type Tool = 'move' | 'frame' | 'text' | 'pen' | Shape
export type Pen = { id: string; anchors: Anchor[] }

export const MM = 72 / 25.4

export class Editor {
  snapshot: Snapshot
  nodes: Map<string, Entry>
  selection: string[] = []
  tool: Tool = 'move'
  renaming: string | null = null
  /** The path being drawn with the pen tool, inside an open undo group. */
  pen: Pen | null = null
  private listeners = new Set<() => void>()

  constructor(readonly engine: Engine) {
    this.snapshot = engine.snapshot()
    this.nodes = index(this.page.children)
  }

  get page() {
    return this.snapshot.pages[0]
  }

  apply(cmd: Command): string[] {
    const ids = this.engine.apply(cmd)
    this.snapshot = this.engine.snapshot()
    this.nodes = index(this.page.children)
    this.selection = this.selection.filter((id) => this.nodes.has(id))
    this.emit()
    return ids
  }

  set(patch: Partial<Pick<Editor, 'selection' | 'tool' | 'renaming' | 'pen'>>) {
    Object.assign(this, patch)
    this.emit()
  }

  setTool(tool: Tool) {
    this.finishPen(false)
    this.set({ tool })
  }

  /** Ends the pen path; a path with fewer than two anchors is removed. */
  finishPen(closed: boolean) {
    const pen = this.pen
    if (!pen) return
    this.pen = null
    if (pen.anchors.length < 2) this.apply({ type: 'delete', ids: [pen.id] })
    else this.apply({ type: 'setPath', id: pen.id, path: penPath(pen.anchors, closed) })
    this.apply({ type: 'endUndoGroup' })
    this.set({ tool: 'move', selection: pen.anchors.length < 2 ? [] : [pen.id] })
  }

  selected(): Node[] {
    return this.selection.flatMap((id) => this.nodes.get(id)?.node ?? [])
  }

  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private emit() {
    for (const l of this.listeners) l()
  }
}

export function useEditor<T>(editor: Editor, read: (e: Editor) => T): T {
  return useSyncExternalStore(editor.subscribe, () => read(editor))
}

export type Point = { x: number; y: number }

/** Start and end in pt of a line or arrow, i.e. a two-point path. */
export function ends(n: Node): [Point, Point] | undefined {
  if (n.kind !== 'shape' || n.shape !== 'path' || n.path.length !== 6) return undefined
  const [, u0, v0, , u1, v1] = n.path
  return [
    { x: n.x + u0 * n.w, y: n.y + v0 * n.h },
    { x: n.x + u1 * n.w, y: n.y + v1 * n.h },
  ]
}

export function bounds(nodes: { x: number; y: number; w: number; h: number }[]) {
  const x = Math.min(...nodes.map((n) => n.x))
  const y = Math.min(...nodes.map((n) => n.y))
  const w = Math.max(...nodes.map((n) => n.x + n.w)) - x
  const h = Math.max(...nodes.map((n) => n.y + n.h)) - y
  return { x, y, w, h }
}
