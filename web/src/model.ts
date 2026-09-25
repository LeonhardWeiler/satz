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
  constraints: { horizontal: Constraint; vertical: Constraint }
}

/** How a frame's child follows the frame on resize: pinned to the start, the end, both, the centre, or scaled. */
export type Constraint = 'min' | 'max' | 'stretch' | 'center' | 'scale'

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
  | { type: 'addSwatch'; name: string; color: Color; spot: boolean }
  | { type: 'setSwatch'; id: string; name?: string; color?: Color; spot?: boolean }
  | { type: 'deleteSwatch'; id: string }
  | { type: 'addCollection'; name: string }
  | { type: 'setCollection'; id: string; name: string }
  | { type: 'deleteCollection'; id: string }
  | { type: 'addMode'; collection: string; name: string }
  | { type: 'setMode'; collection: string; id: string; name: string }
  | { type: 'deleteMode'; collection: string; id: string }
  | { type: 'addVariable'; collection: string; name: string; value: Value }
  | { type: 'setVariable'; id: string; name?: string; mode?: string; value?: Value }
  | { type: 'deleteVariable'; id: string }
  | { type: 'useMode'; id: string; collection: string; mode: string | null }
  | { type: 'bind'; id: string; prop: Bindable; variable: string | null }
  | { type: 'undo' | 'redo' | 'beginUndoGroup' | 'endUndoGroup' }

export type Node = {
  id: string
  name: string
  x: number
  y: number
  w: number
  h: number
  modes: Modes
  activeModes: Modes
  bindings: Partial<Record<Bindable, string>>
} & Style &
  (
    | ({ kind: 'shape' } & Shape)
    | { kind: 'text'; text: string; size: number }
    | { kind: 'group'; children: Node[] }
    | { kind: 'frame'; clip: boolean; children: Node[] }
  )

export type Container = Extract<Node, { children: Node[] }>

export type Page = { id: string; width: number; height: number; bleed: number; modes: Modes; children: Node[] }

/** A spot colour's `color` is its CMYK alternate. */
export type Swatch = { id: string; name: string; color: Color; spot: boolean }

/** Chosen mode per collection; collections not listed use their first mode. */
export type Modes = Record<string, string>
export type Collection = { id: string; name: string; modes: { id: string; name: string }[] }
export type Value = { color: Color } | { number: number }
/** One value per mode of its collection, all of one kind. */
export type Variable = { id: string; collection: string; name: string; values: Record<string, Value> }
export type Palette = { swatches: Swatch[]; collections: Collection[]; variables: Variable[] }
/** A palette seen from a layer with its modes. */
export type Scope = Palette & { modes: Modes }
/** Lengths count in mm, opacity in %. */
export type Bindable = 'w' | 'h' | 'radius' | 'strokeWeight' | 'opacity'

export type Snapshot = Palette & {
  pages: Page[]
  rasterPpi: number
  colorMode: ColorMode
  canUndo: boolean
  canRedo: boolean
}
