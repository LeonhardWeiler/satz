export type Command =
  | { type: 'create'; parent: string; kind: 'rect' | 'text' | 'frame'; x: number; y: number; w: number; h: number }
  | { type: 'setFrame'; id: string; x: number; y: number; w: number; h: number }
  | { type: 'setText'; id: string; text: string }
  | { type: 'set'; id: string; name?: string; fill?: number; size?: number; clip?: boolean }
  | { type: 'delete'; ids: string[] }
  | { type: 'group'; ids: string[]; frame: boolean }
  | { type: 'ungroup'; ids: string[] }
  | { type: 'move'; ids: string[]; parent: string; index: number }
  | { type: 'order'; ids: string[]; to: 'forward' | 'backward' | 'front' | 'back' }
  | { type: 'duplicate'; ids: string[] }
  | { type: 'undo' | 'redo' | 'beginUndoGroup' | 'endUndoGroup' }

export type Node = { id: string; name: string; x: number; y: number; w: number; h: number } & (
  | { kind: 'rect'; fill: number }
  | { kind: 'text'; text: string; size: number }
  | { kind: 'group'; children: Node[] }
  | { kind: 'frame'; fill: number; clip: boolean; children: Node[] }
)

export type Page = { id: string; width: number; height: number; bleed: number; children: Node[] }

export type Snapshot = { pages: Page[]; canUndo: boolean; canRedo: boolean }
