import type { Node, Palette, Sizing, Span } from './types'

export type * from './types'

export type Container = Extract<Node, { children: Node[] }>
export type TextNode = Extract<Node, { kind: 'text' }>
/** Chosen mode per collection; collections not listed use their first mode. */
export type Modes = Record<string, string>
/** A palette seen from a layer with its modes. */
export type Scope = Omit<Palette, 'textStyles'> & { modes: Modes }
/** Lengths count in mm, opacity and letter spacing in %, type in pt. */
export type Bindable =
  | 'font' | 'w' | 'h' | 'radius' | 'strokeWeight' | 'opacity'
  | 'gap' | 'paddingTop' | 'paddingRight' | 'paddingBottom' | 'paddingLeft'
  | Styled
export type Attrs = Omit<Span, 'len'>
export type TextProps = Partial<Attrs>
/** The attributes a text style sets. */
export const STYLED = ['size', 'lineHeight', 'letterSpacing', 'paragraphSpacing', 'paragraphIndent', 'indentLeft', 'indentRight', 'spaceBefore', 'font', 'textCase', 'textDecoration', 'features', 'axes', 'position', 'baselineShift', 'dropLines', 'dropChars', 'keepLines', 'keepTogether', 'keepNext', 'tabs', 'list'] as const
export type Styled = (typeof STYLED)[number]

/** The radii of a rectangle's top left, top right, bottom right and bottom left corner. */
export type Resizing = 'autoWidth' | 'autoHeight' | 'fixedSize'
/** Figma's text resize modes as sizing: auto width hugs both sides, auto height the height. */
export function textSizing({ horizontal, vertical }: Sizing, mode: Resizing): Sizing {
  const width = horizontal === 'hug' ? 'fixed' : horizontal
  return mode === 'autoWidth' ? { horizontal: 'hug', vertical: 'hug' }
    : mode === 'autoHeight' ? { horizontal: width, vertical: 'hug' }
    : { horizontal: width, vertical: vertical === 'fill' ? 'fill' : 'fixed' }
}

export const isOpen = (n: Node) => n.kind === 'shape' && n.shape === 'path' && !n.path.includes(5)

type Rounded = { radius: number; corners?: number[] }
/** Whether `n` has corners to round: a frame or a rectangle. */
export const rounded = (n: Node): n is Node & Rounded => n.kind === 'frame' || (n.kind === 'shape' && n.shape === 'rect')
export const radii = ({ radius, corners }: Rounded) => (corners?.length ? corners : [radius, radius, radius, radius])
/** The radius of all corners of a rectangle, or null if they differ. */
export const radiusOf = (s: Rounded) => {
  const r = radii(s)
  return r.every((v) => v === r[0]) ? r[0] : null
}
