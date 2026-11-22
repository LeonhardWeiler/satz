import type { ReactNode } from 'react'
import { bounds, MM, type Editor } from './editor'
import { Icon, type IconName } from './icons'
import type { BooleanOp, Node } from './model'
import { neighbours } from './snap'

export const BOOLEANS: [BooleanOp, string, IconName, string][] = [
  ['union', 'Union', 'union', 'Ctrl Alt U'],
  ['subtract', 'Subtract', 'subtract', 'Ctrl Alt S'],
  ['intersect', 'Intersect', 'intersect', 'Ctrl Alt I'],
  ['exclude', 'Exclude', 'exclude', 'Ctrl Alt X'],
]

export const combinable = (editor: Editor) => editor.selection.length > 1 && editor.selected().every((n) => n.kind === 'shape')

export function combine(editor: Editor, op: BooleanOp) {
  if (combinable(editor)) editor.set({ selection: editor.apply({ type: 'boolean', ids: editor.selection, op }) })
}

export type Align = 'left' | 'center' | 'right' | 'top' | 'middle' | 'bottom' | 'distributeX' | 'distributeY' | 'tidy'

export const ALIGNS: [Align, string, IconName, string][] = [
  ['left', 'Align left', 'alignLeftEdges', 'Alt A'],
  ['center', 'Align horizontal centers', 'alignCenters', 'Alt H'],
  ['right', 'Align right', 'alignRightEdges', 'Alt D'],
  ['top', 'Align top', 'alignTopEdges', 'Alt W'],
  ['middle', 'Align vertical centers', 'alignMiddles', 'Alt V'],
  ['bottom', 'Align bottom', 'alignBottomEdges', 'Alt S'],
  ['distributeX', 'Distribute horizontal spacing', 'distributeX', 'Alt Shift H'],
  ['distributeY', 'Distribute vertical spacing', 'distributeY', 'Alt Shift V'],
  ['tidy', 'Tidy up', 'tidy', 'Ctrl Alt Shift T'],
]

const frameOf = (editor: Editor, id?: string) => {
  let parent = id ? editor.nodes.get(id)?.parent : undefined
  while (parent?.kind === 'group') parent = editor.nodes.get(parent.id)?.parent
  return parent?.kind === 'frame' ? parent : undefined
}

export function align(editor: Editor, how: Align, layers = editor.selected()) {
  const nodes = layers.filter((n) => !n.locked)
  const to = nodes.length > 1 ? bounds(nodes) : frameOf(editor, nodes[0]?.id) ?? { x: 0, y: 0, w: editor.page.width, h: editor.page.height }
  const moves = new Map<Node, [number, number]>()
  const sorted = (x: 'x' | 'y', w: 'w' | 'h') => [...nodes].sort((a, b) => a[x] + a[w] / 2 - b[x] - b[w] / 2)
  const spread = (x: 'x' | 'y', w: 'w' | 'h') => {
    const s = sorted(x, w)
    const gap = (to[w] - s.reduce((t, n) => t + n[w], 0)) / (s.length - 1)
    let at = to[x]
    for (const n of s) {
      moves.set(n, x === 'x' ? [at, n.y] : [n.x, at])
      at += n[w] + gap
    }
  }
  if (how === 'distributeX' || how === 'distributeY') {
    if (nodes.length < 3) return
    if (how === 'distributeX') spread('x', 'w')
    else spread('y', 'h')
  } else if (how === 'tidy') {
    if (nodes.length < 2) return
    const rows: Node[][] = []
    for (const n of sorted('y', 'h')) {
      const row = rows.at(-1)
      if (row && n.y + n.h / 2 < Math.max(...row.map((r) => r.y + r.h))) row.push(n)
      else rows.push([n])
    }
    for (const row of rows) row.sort((a, b) => a.x - b.x)
    const cw = Math.max(...nodes.map((n) => n.w))
    const rh = Math.max(...nodes.map((n) => n.h))
    const gaps = rows.flatMap((row) => neighbours(row).map(([m, n]) => n.x - m.x - m.w)).filter((g) => g > 0)
    const gap = gaps.length ? Math.min(...gaps) : 5 * MM
    rows.forEach((row, r) => row.forEach((n, c) => moves.set(n, [to.x + c * (cw + gap), to.y + r * (rh + gap)])))
  } else {
    for (const n of nodes) {
      const x = { left: to.x, center: to.x + (to.w - n.w) / 2, right: to.x + to.w - n.w }[how as string] ?? n.x
      const y = { top: to.y, middle: to.y + (to.h - n.h) / 2, bottom: to.y + to.h - n.h }[how as string] ?? n.y
      moves.set(n, [x, y])
    }
  }
  editor.batch(() => {
    for (const [n, [x, y]] of moves) if (x !== n.x || y !== n.y) editor.apply({ type: 'setFrame', id: n.id, x, y, w: n.w, h: n.h })
  })
}

export type Row = { axis: 'x' | 'y'; nodes: Node[]; gaps: number[] }

export function rowOf(editor: Editor): Row | null {
  const nodes = editor.selected()
  const parents = new Set(nodes.map((n) => editor.nodes.get(n.id)?.parent))
  const [parent] = parents
  if (nodes.length < 2 || parents.size > 1 || (parent?.kind === 'frame' && parent.direction !== 'none') || nodes.some((n) => n.locked || n.rotation)) return null
  for (const [x, w, y, h] of [['x', 'w', 'y', 'h'], ['y', 'h', 'x', 'w']] as const) {
    const s = [...nodes].sort((a, b) => a[x] - b[x])
    const pairs = neighbours(s)
    const gaps = pairs.map(([m, n]) => n[x] - m[x] - m[w])
    if (gaps.every((g) => g >= 0) && pairs.every(([m, n]) => n[y] < m[y] + m[h] && m[y] < n[y] + n[h])) return { axis: x, nodes: s, gaps }
  }
  return null
}

export function setGap(editor: Editor, { axis, nodes }: Row, gap: number) {
  const w = axis === 'x' ? 'w' : 'h'
  let at = nodes[0]?.[axis] ?? 0
  editor.batch(() => {
    for (const n of nodes) {
      if (n[axis] !== at) editor.apply({ type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w, h: n.h, [axis]: at })
      at += n[w] + gap
    }
  })
}

export function AlignBar({ editor, inside, children }: { editor: Editor; inside?: Node[]; children?: ReactNode }) {
  const n = inside?.length ?? editor.selection.length
  const button = ([how, title, icon]: (typeof ALIGNS)[number]) => (
    <button
      key={how}
      type="button"
      className="icon-button"
      aria-label={title}
      title={inside ? `${title} inside the group` : n > 1 ? title : `${title} to the ${frameOf(editor, editor.selection[0]) ? 'frame' : 'page'}`}
      data-action={inside ? undefined : title}
      onClick={() => align(editor, how, inside)}
    >
      <Icon name={icon} />
    </button>
  )
  const spread = !inside && n >= 3
  const bar = (
    <div role="toolbar" aria-label={inside ? 'Align inside' : 'Align'} className="align">
      {ALIGNS.slice(0, 6).map(button)}
    </div>
  )
  if (inside)
    return (
      <div role="group" aria-label="Inside the group" className="options-group">
        <header>
          <h4>Inside the group</h4>
        </header>
        {bar}
      </div>
    )
  return (
    <>
      {bar}
      {(spread || children) && (
        <div role="toolbar" aria-label="Distribute" className="align">
          {spread && ALIGNS.slice(6).map(button)}
          {children}
        </div>
      )}
    </>
  )
}
