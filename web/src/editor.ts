import { useSyncExternalStore } from 'react'
import type { Engine } from './engine/engine'
import type { Command, Container, Modes, Node, Page, Palette, Scope, Snapshot } from './model'
import { penPath, type Anchor } from './pen'
import { index, type Entry } from './select'
import type { Editing } from './textEdit'

export type Shape = 'rect' | 'line' | 'arrow' | 'ellipse' | 'polygon' | 'star'
export type Tool = 'move' | 'frame' | 'text' | 'pen' | Shape
export type Pen = { id: string; anchors: Anchor[] }

export const MM = 72 / 25.4

export const scopeOf = ({ swatches, collections, variables }: Palette, modes: Modes = {}): Scope => ({
  swatches,
  collections,
  variables,
  modes,
})

export class Editor {
  snapshot: Snapshot
  nodes: Map<string, Entry>
  selection: string[] = []
  tool: Tool = 'move'
  renaming: string | null = null
  /** The path being drawn with the pen tool, inside an open undo group. */
  pen: Pen | null = null
  /** The pointer is down on the canvas, possibly inside a drag's undo group. */
  dragging = false
  /** The text layer edited in its frame, which is then the selection. */
  editing: Editing | null = null
  /** The page shown on the canvas. */
  pageId: string
  /** Typing into the edited text is one undo step until the caret moves. */
  private typing = false
  private listeners = new Set<() => void>()

  constructor(readonly engine: Engine) {
    this.snapshot = engine.snapshot()
    this.pageId = this.snapshot.pages[0].id
    this.nodes = index(this.page.children)
  }

  /** The page or master shown on the canvas. */
  get page() {
    const { pages, masters } = this.snapshot
    return pages.find((p) => p.id === this.pageId) ?? masters.find((p) => p.id === this.pageId) ?? pages[0]
  }

  /** The master a page draws under its layers. */
  masterOf(page: Page) {
    return this.snapshot.masters.find((m) => m.id === page.master)
  }

  apply(cmd: Command): string[] {
    const at = this.snapshot.pages.findIndex((p) => p.id === this.pageId)
    try {
      return this.engine.apply(cmd)
    } finally {
      this.snapshot = this.engine.snapshot()
      const { pages, masters } = this.snapshot
      if (!pages.some((p) => p.id === this.pageId) && !masters.some((p) => p.id === this.pageId)) {
        this.pageId = pages[Math.max(0, Math.min(at, pages.length - 1))].id
      }
      this.nodes = index(this.page.children)
      this.selection = this.selection.filter((id) => this.nodes.has(id))
      const n = this.editing && this.nodes.get(this.editing.id)?.node
      if (this.editing) {
        const len = n?.kind === 'text' ? n.text.length : -1
        const { anchor, focus } = this.editing
        this.editing = len < 0 ? null : { ...this.editing, anchor: Math.min(anchor, len), focus: Math.min(focus, len) }
      }
      this.emit()
    }
  }

  set(patch: Partial<Pick<Editor, 'selection' | 'tool' | 'renaming' | 'pen' | 'editing'>>) {
    const leaves = this.editing && patch.selection && !patch.selection.includes(this.editing.id)
    if (leaves && !('editing' in patch)) this.stopEditing()
    if (patch.editing) patch = { selection: [patch.editing.id], ...patch }
    Object.assign(this, patch)
    this.emit()
  }

  /** Shows the page `id` with nothing selected. */
  showPage(id: string) {
    if (id === this.pageId) return
    this.finishPen(false)
    this.stopEditing()
    this.pageId = id
    this.nodes = index(this.page.children)
    this.set({ selection: [] })
  }

  setTool(tool: Tool) {
    this.finishPen(false)
    this.stopEditing()
    this.set({ tool })
  }

  beginTyping() {
    if (this.typing) return
    this.typing = true
    this.apply({ type: 'beginUndoGroup' })
  }

  endTyping() {
    if (!this.typing) return
    this.typing = false
    this.apply({ type: 'endUndoGroup' })
  }

  /** Leaves the edited text selected, or removes it when it is empty, as Figma does. */
  stopEditing() {
    const e = this.editing
    if (!e) return
    this.endTyping()
    this.editing = null
    const n = this.nodes.get(e.id)?.node
    if (n?.kind === 'text' && !n.text) this.apply({ type: 'delete', ids: [e.id] })
    this.emit()
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

  /** Makes the pointer gesture that starts now one undo step. */
  gesture = () => {
    this.finishPen(false)
    this.endTyping()
    this.apply({ type: 'beginUndoGroup' })
    window.addEventListener('pointerup', () => this.apply({ type: 'endUndoGroup' }), { once: true })
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

/**
 * Where layers `ids` dragged to `p` land among the other children of the auto layout
 * frame `parent`: the index for a move and the insertion line.
 */
export function insertion(parent: Container, ids: string[], p: Point) {
  const horizontal = parent.kind === 'frame' && parent.direction === 'horizontal'
  const others = parent.children.filter((n) => !ids.includes(n.id))
  const flow = others.filter((n) => !n.absolute)
  const main = (n: Node) => (horizontal ? n.x + n.w / 2 : n.y + n.h / 2)
  const next = flow.find((n) => main(n) > (horizontal ? p.x : p.y))
  const index = next ? others.indexOf(next) : others.length
  const prev = flow[(next ? flow.indexOf(next) : flow.length) - 1]
  const at = next && prev ? (horizontal ? (prev.x + prev.w + next.x) / 2 : (prev.y + prev.h + next.y) / 2)
    : next ? (horizontal ? next.x : next.y)
    : prev ? (horizontal ? prev.x + prev.w : prev.y + prev.h)
    : horizontal ? parent.x : parent.y
  const line: [Point, Point] = horizontal
    ? [{ x: at, y: parent.y }, { x: at, y: parent.y + parent.h }]
    : [{ x: parent.x, y: at }, { x: parent.x + parent.w, y: at }]
  return { index, line }
}
