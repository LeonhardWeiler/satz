export type Command =
  | { type: 'setFrame'; id: string; x: number; y: number; w: number; h: number }
  | { type: 'setText'; id: string; text: string }

export type Item = { id: string; x: number; y: number; w: number; h: number } & (
  | { kind: 'rect'; fill: number }
  | { kind: 'text'; text: string }
)

export type Page = { width: number; height: number; bleed: number; items: Item[] }

export type Snapshot = { pages: Page[] }
