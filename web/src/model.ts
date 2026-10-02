import type { BooleanOp } from './align'
import type { Color, ColorMode } from './color'

/**
 * Gradients map their unit space, and images the unit square they fill, into the
 * layer's unit box with `transform`; `image` is the hash of an image fill's file,
 * and `adjust` its brightness, contrast and saturation in -1..1.
 */
export type Fill = {
  type: 'solid' | 'linear' | 'radial' | 'image'
  color: Color
  stops: { at: number; color: Color }[]
  transform: number[]
  visible: boolean
  image?: string
  adjust?: [number, number, number]
}

export type Effect = { type: 'dropShadow' | 'blur'; x: number; y: number; radius: number; color: Color; visible: boolean }

export type Blend =
  | 'normal' | 'multiply' | 'screen' | 'overlay' | 'darken' | 'lighten' | 'colorDodge' | 'colorBurn'
  | 'hardLight' | 'softLight' | 'difference' | 'exclusion' | 'hue' | 'saturation' | 'color' | 'luminosity'

export type LineStyle = 'solid' | 'dashed' | 'dotted' | 'wavy' | 'zigzag'

export type Style = {
  fills: Fill[]
  strokes: Fill[]
  strokeWeight: number
  strokeAlign: 'inside' | 'center' | 'outside'
  join: 'miter' | 'round' | 'bevel'
  cap: 'none' | 'round' | 'square'
  lineStyle: LineStyle
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
  | { shape: 'rect'; radius: number; corners?: number[] }
  | { shape: 'ellipse'; start: number; sweep: number; inner: number }
  | { shape: 'polygon'; count: number }
  | { shape: 'star'; count: number; ratio: number }
  | { shape: 'path'; path: number[] }

export type Direction = 'none' | 'horizontal' | 'vertical'
/** Hug only applies to auto layout frames and text, fill only inside auto layout frames. */
export type Size = 'fixed' | 'hug' | 'fill'
/** On text, hugging both sides is auto width and hugging the height auto height. */
export type Sizing = { horizontal: Size; vertical: Size }

/** Figma auto layout of a frame and how a layer sits in one. */
export type Layout = {
  direction: Direction
  gap: number
  paddingTop: number
  paddingRight: number
  paddingBottom: number
  paddingLeft: number
  alignMain: 'start' | 'center' | 'end' | 'spaceBetween'
  alignCross: 'start' | 'center' | 'end'
  sizing: Sizing
  /** Left out of its parent's auto layout. */
  absolute: boolean
}

export type Props = Partial<Style> & Partial<Layout> & Partial<TextFrame> & { name?: string; hidden?: boolean; locked?: boolean; rotation?: number; clip?: boolean; radius?: number; corners?: number[]; start?: number; sweep?: number; inner?: number; count?: number; ratio?: number }

export type NewKind = 'rect' | 'ellipse' | 'polygon' | 'star' | 'line' | 'path' | 'text' | 'frame'

export type Command =
  | { type: 'create'; parent: string; kind: NewKind; x: number; y: number; w: number; h: number }
  | { type: 'placeImage'; parent: string; image: string; name: string; x: number; y: number; w: number; h: number }
  | { type: 'setFrame'; id: string; x: number; y: number; w: number; h: number; crop?: boolean }
  | { type: 'autoLayout'; ids: string[] }
  | { type: 'setText'; id: string; text: string }
  | { type: 'editText'; id: string; range: [number, number]; text: string }
  | ({ type: 'format'; id: string; range: [number, number] | null } & TextProps)
  | ({ type: 'addTextStyle'; name: string } & Partial<Pick<Attrs, Styled>>)
  | ({ type: 'setTextStyle'; id: string; name?: string } & Partial<Pick<Attrs, Styled>>)
  | { type: 'deleteTextStyle'; id: string }
  | ({ type: 'set'; id: string } & Props)
  | { type: 'setPath'; id: string; path: number[] }
  | { type: 'delete'; ids: string[] }
  | { type: 'group'; ids: string[]; frame: boolean }
  | { type: 'ungroup'; ids: string[] }
  | { type: 'mask'; ids: string[] }
  | { type: 'move'; ids: string[]; parent: string; index: number }
  | { type: 'order'; ids: string[]; to: 'forward' | 'backward' | 'front' | 'back' }
  | { type: 'duplicate' | 'copy'; ids: string[] }
  | { type: 'paste'; above: string[]; page?: string }
  | { type: 'addPage'; after: string | null }
  | { type: 'duplicatePage' | 'deletePage'; id: string }
  | { type: 'setPage'; id: string; width?: number; height?: number; bleed?: number; scale?: boolean }
  | { type: 'setGrids'; id: string; grids: Grid[] }
  | { type: 'setGuides'; id: string; guides: Guides }
  | { type: 'flatten'; id: string }
  | { type: 'flip'; id: string; vertical: boolean }
  | { type: 'boolean'; ids: string[]; op: BooleanOp }
  | { type: 'fillArea'; id: string; x: number; y: number }
  | { type: 'scaleText'; id: string; by: number }
  | { type: 'movePage'; id: string; index: number }
  | { type: 'addMaster'; like: string | null }
  | { type: 'setMaster'; id: string; name: string }
  | { type: 'deleteMaster'; id: string }
  | { type: 'useMaster'; page: string; master: string | null }
  | { type: 'override'; page: string; id: string }
  | { type: 'resetToMaster'; ids: string[] }
  | { type: 'thread'; from: string; to: string }
  | { type: 'unthread'; id: string }
  | { type: 'setDocument'; rasterPpi?: number; colorMode?: ColorMode; facingPages?: boolean; inkLimit?: number; preset?: Preset; cropMarks?: boolean; includeBleed?: boolean }
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
  /** The master layer this page layer overrides. */
  overrideOf?: string
  /** Effective pixels per inch of the coarsest visible image fill. */
  ppi?: number
  /** Not drawn, exported, hit or preflighted, with its children. */
  hidden: boolean
  /** Not hit on the canvas, with its children. */
  locked: boolean
  /** Degrees counterclockwise around the centre, with the children. */
  rotation: number
  /** The upright box on the page that the layer and what it does not clip cover, turned by its rotation and its ancestors'. */
  bounds: [number, number, number, number]
} & Style &
  Layout &
  (
    | ({ kind: 'shape' } & Shape)
    | ({ kind: 'text' } & Threaded & TextFrame)
    | { kind: 'group'; children: Node[] }
    | { kind: 'frame'; clip: boolean; children: Node[] }
  )

export type Container = Extract<Node, { children: Node[] }>
export type TextNode = Extract<Node, { kind: 'text' }>

/** The text of a thread and its runs of equal attributes. */
export type Story = { text: string; spans: Span[] }

/**
 * A text layer in its thread, whose story is held by its first frame `story`; the
 * layer sets it from `start` to `end` in UTF-16, and is `overset` when it is the last
 * frame and text is left.
 */
export type Threaded = {
  story: string
  start: number
  end: number
  prev: string | null
  next: string | null
  overset: boolean
}

/**
 * A page or a master: `name` is a master's, `master` the one a page draws under its
 * layers and `detached` the master layers it overrides; `side` is the side of its
 * spread a page is on with facing pages.
 */
/** Ruler guides of a page: vertical ones at `x` from its left edge, horizontal ones at `y` from its top. */
export type Guides = { x: number[]; y: number[] }

export type Page = {
  id: string
  name: string
  width: number
  height: number
  bleed: number
  side: 'left' | 'right' | null
  /** Where the page's left edge sits on its spread, whose spine is at 0. */
  x: number
  master: string | null
  detached: string[]
  grids: Grid[]
  guides: Guides
  modes: Modes
  children: Node[]
}

/** `count` columns or rows between margins and gutters, or square cells of `size`, in pt. */
export type Grid = { kind: 'columns' | 'rows' | 'grid'; count: number; gutter: number; margin: number; size: number }

/** A spot colour's `color` is its CMYK alternate. */
export type Swatch = { id: string; name: string; color: Color; spot: boolean }

/** Chosen mode per collection; collections not listed use their first mode. */
export type Modes = Record<string, string>
export type Collection = { id: string; name: string; modes: { id: string; name: string }[] }
/** A font `null` is the bundled one. */
export type Value = { color: Color } | { number: number } | { font: Typeface | null }
/** One value per mode of its collection, all of one kind. */
export type Variable = { id: string; collection: string; name: string; values: Record<string, Value> }
export type Palette = { swatches: Swatch[]; collections: Collection[]; variables: Variable[]; textStyles: TextStyle[] }
/** A palette seen from a layer with its modes. */
export type Scope = Omit<Palette, 'textStyles'> & { modes: Modes }
/** Lengths count in mm, opacity and letter spacing in %, type in pt. */
export type Bindable =
  | 'font' | 'w' | 'h' | 'radius' | 'strokeWeight' | 'opacity'
  | 'gap' | 'paddingTop' | 'paddingRight' | 'paddingBottom' | 'paddingLeft'
  | Styled

/**
 * What a character looks like. Sizes and spacing in pt, letter spacing in % of the size;
 * line height 0 is auto. `fill` replaces the layer's fills; `textStyle` '' means none.
 * Paragraph attributes come from each paragraph's first character.
 */
export type Attrs = {
  size: number
  lineHeight: number
  letterSpacing: number
  paragraphSpacing: number
  fill: Color | null
  textStyle: string
  textAlign: 'left' | 'center' | 'right' | 'justify'
  hyphenate: boolean
  lang: 'en' | 'de'
  /** `null` is the bundled font. */
  font: Typeface | null
  paragraphIndent: number
  textCase: 'original' | 'upper' | 'lower' | 'title'
  textDecoration: 'none' | 'underline' | 'strikethrough'
  /** OpenType features as harfrust parses them, e.g. "smcp" or "liga=0". */
  features: string[]
}
/** A font by its full name, family and style, and a hash of its bytes. */
export type Typeface = { name: string; family: string; style: string; hash: string }
export type TextProps = Partial<Attrs>
/**
 * How a text layer sets its text: insets and gutter in pt, columns of equal width, and
 * baselines on a grid of `baselineGrid` pt from `baselineStart` below the top inset; 0 is off.
 */
export type TextFrame = {
  insetTop: number
  insetRight: number
  insetBottom: number
  insetLeft: number
  columns: number
  gutter: number
  verticalAlign: 'top' | 'center' | 'bottom'
  baselineGrid: number
  baselineStart: number
}
/** `len` characters in UTF-16 code units that share their attributes. */
export type Span = Attrs & { len: number }
/** The attributes a text style sets. */
export const STYLED = ['size', 'lineHeight', 'letterSpacing', 'paragraphSpacing', 'paragraphIndent', 'font', 'textCase', 'textDecoration', 'features'] as const
export type Styled = (typeof STYLED)[number]
export type TextStyle = { id: string; name: string; bindings: Partial<Record<Bindable, string>> } & Pick<Attrs, Styled>

/** Something about the layer `layer` on the page or master `page` that may print wrong. */
export type Issue = { page: string; layer: string; name: string } & (
  | { problem: 'overset' | 'shortOfBleed' | 'rgb' | 'gamut' }
  | { problem: 'missingFont'; font: string }
  | { problem: 'lowPpi'; ppi: number }
  | { problem: 'ink'; ink: number }
  | { problem: 'nearTrim'; distance: number }
)

export type Preset = 'screen' | 'x4' | 'x1a'
export type Snapshot = Palette & {
  pages: Page[]
  masters: Page[]
  facingPages: boolean
  /** The ids of the pages of each spread from left to right. */
  spreads: string[][]
  /** The story of each thread by its first frame. */
  stories: Record<string, Story>
  rasterPpi: number
  colorMode: ColorMode
  inkLimit: number
  /** How the PDF exports; PDF/X-1a only in CMYK documents. */
  preset: Preset
  cropMarks: boolean
  includeBleed: boolean
  /** The fonts text can be set in, the bundled one first. */
  fonts: Typeface[]
  /** Fonts text is set in that are not there, with the stories that use them by their first frame. */
  missingFonts: { font: Typeface; stories: string[] }[]
  /** The colours of each image by its hash. */
  images: Record<string, 'Gray' | 'RGB' | 'CMYK'>
  preflight: Issue[]
  canUndo: boolean
  canRedo: boolean
}

/** The radii of a rectangle's top left, top right, bottom right and bottom left corner. */
export type Resizing = 'autoWidth' | 'autoHeight' | 'fixedSize'
/** Figma's text resize modes as sizing: auto width hugs both sides, auto height the height. */
export function textSizing({ horizontal, vertical }: Sizing, mode: Resizing): Sizing {
  const width = horizontal === 'hug' ? 'fixed' : horizontal
  return mode === 'autoWidth' ? { horizontal: 'hug', vertical: 'hug' }
    : mode === 'autoHeight' ? { horizontal: width, vertical: 'hug' }
    : { horizontal: width, vertical: vertical === 'fill' ? 'fill' : 'fixed' }
}

export const radii = ({ radius, corners }: { radius: number; corners?: number[] }) => (corners?.length ? corners : [radius, radius, radius, radius])
/** The radius of all corners of a rectangle, or null if they differ. */
export const isOpen = (n: Node) => n.kind === 'shape' && n.shape === 'path' && !n.path.includes(5)

export const radiusOf = (s: { radius: number; corners?: number[] }) => {
  const r = radii(s)
  return r.every((v) => v === r[0]) ? r[0] : null
}
