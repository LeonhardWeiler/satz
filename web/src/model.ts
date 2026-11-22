import type { Node, Sizing, Span } from './types'

export type * from './types'

export type Container = Extract<Node, { children: Node[] }>
export type TextNode = Extract<Node, { kind: 'text' }>
export type Modes = Record<string, string>
export type Attrs = Omit<Span, 'len'>
export type TextProps = Partial<Attrs>
export { BINDABLE, STYLED } from './types'

export type Resizing = 'autoWidth' | 'autoHeight' | 'fixedSize'
export function textSizing({ horizontal, vertical }: Sizing, mode: Resizing): Sizing {
  const width = horizontal === 'hug' ? 'fixed' : horizontal
  return mode === 'autoWidth' ? { horizontal: 'hug', vertical: 'hug' }
    : mode === 'autoHeight' ? { horizontal: width, vertical: 'hug' }
    : { horizontal: width, vertical: vertical === 'fill' ? 'fill' : 'fixed' }
}

export const isOpen = (n: Node) => n.kind === 'shape' && n.shape === 'path' && !n.path.includes(5)

type Rounded = { radius: number; corners?: number[] }
export const rounded = (n: Node): n is Node & Rounded => n.kind === 'frame' || (n.kind === 'shape' && n.shape === 'rect')
export const radii = ({ radius, corners }: Rounded) => (corners?.length ? corners : [radius, radius, radius, radius])
export const radiusOf = (s: Rounded) => {
  const r = radii(s)
  return r.every((v) => v === r[0]) ? r[0] : null
}
