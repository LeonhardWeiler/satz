import { useSyncExternalStore } from 'react'
import type { Engine } from './engine/engine'
import type { Command, Node, Snapshot } from './model'
import { index, type Entry } from './select'

export type Tool = 'move' | 'frame' | 'rect' | 'text'

export const MM = 72 / 25.4

export class Editor {
  snapshot: Snapshot
  nodes: Map<string, Entry>
  selection: string[] = []
  tool: Tool = 'move'
  renaming: string | null = null
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

  set(patch: Partial<Pick<Editor, 'selection' | 'tool' | 'renaming'>>) {
    Object.assign(this, patch)
    this.emit()
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

export function bounds(nodes: { x: number; y: number; w: number; h: number }[]) {
  const x = Math.min(...nodes.map((n) => n.x))
  const y = Math.min(...nodes.map((n) => n.y))
  const w = Math.max(...nodes.map((n) => n.x + n.w)) - x
  const h = Math.max(...nodes.map((n) => n.y + n.h)) - y
  return { x, y, w, h }
}
