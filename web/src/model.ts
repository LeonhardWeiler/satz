import type { Color, ColorMode } from './color'

export type Fill = {
  type: 'solid' | 'linear' | 'radial'
  color: Color
  stops: { at: number; color: Color }[]
  transform: number[]
  visible: boolean
}

export type Effect = { type: 'dropShadow' | 'blur'; x: number; y: number; radius: number; color: Color; visible: boolean }

export type Blend =
  | 'normal' | 'multiply' | 'screen' | 'overlay' | 'darken' | 'lighten' | 'colorDodge' | 'colorBurn'
  | 'hardLight' | 'softLight' | 'difference' | 'exclusion' | 'hue' | 'saturation' | 'color' | 'luminosity'

export type Style = {
  fills: Fill[]
  strokes: Fill[]
  strokeWeight: number
  strokeAlign: 'inside' | 'center' | 'outside'
  join: 'miter' | 'round' | 'bevel'
  cap: 'none' | 'round' | 'square'
  arrowStart: boolean
  arrowEnd: boolean
  opacity: number
  blend: Blend
  effects: Effect[]
  mask: boolean
}

export type Shape =
  | { shape: 'rect'; radius: number }
  | { shape: 'ellipse' }
  | { shape: 'polygon'; count: number }
  | { shape: 'star'; count: number; ratio: number }
  | { shape: 'path'; path: number[] }

export type Props = Partial<Style> & { name?: string; size?: number; clip?: boolean; radius?: number; count?: number; ratio?: number }

export type NewKind = 'rect' | 'ellipse' | 'polygon' | 'star' | 'line' | 'arrow' | 'path' | 'text' | 'frame'

export type Command =
  | { type: 'create'; parent: string; kind: NewKind; x: number; y: number; w: number; h: number }
  | { type: 'setFrame'; id: string; x: number; y: number; w: number; h: number }
  | { type: 'setText'; id: string; text: string }
  | ({ type: 'set'; id: string } & Props)
  | { type: 'setPath'; id: string; path: number[] }
  | { type: 'delete'; ids: string[] }
  | { type: 'group'; ids: string[]; frame: boolean }
  | { type: 'ungroup'; ids: string[] }
  | { type: 'mask'; ids: string[] }
  | { type: 'move'; ids: string[]; parent: string; index: number }
  | { type: 'order'; ids: string[]; to: 'forward' | 'backward' | 'front' | 'back' }
  | { type: 'duplicate' | 'copy'; ids: string[] }
  | { type: 'paste'; above: string[] }
  | { type: 'setDocument'; rasterPpi?: number; colorMode?: ColorMode }
  | { type: 'undo' | 'redo' | 'beginUndoGroup' | 'endUndoGroup' }

export type Node = { id: string; name: string; x: number; y: number; w: number; h: number } & Style &
  (
    | ({ kind: 'shape' } & Shape)
    | { kind: 'text'; text: string; size: number }
    | { kind: 'group'; children: Node[] }
    | { kind: 'frame'; clip: boolean; children: Node[] }
  )

export type Container = Extract<Node, { children: Node[] }>

export type Page = { id: string; width: number; height: number; bleed: number; children: Node[] }

export type Snapshot = { pages: Page[]; rasterPpi: number; colorMode: ColorMode; canUndo: boolean; canRedo: boolean }
