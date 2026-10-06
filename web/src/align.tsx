import { bounds, MM, type Editor } from './editor'
import { Icon, type IconName } from './icons'
import type { Node } from './model'

export type BooleanOp = 'union' | 'subtract' | 'intersect' | 'exclude'

export const BOOLEANS: [BooleanOp, string, IconName, string][] = [
  ['union', 'Union', 'union', 'Ctrl+Alt+U'],
  ['subtract', 'Subtract', 'subtract', 'Ctrl+Alt+S'],
  ['intersect', 'Intersect', 'intersect', 'Ctrl+Alt+I'],
  ['exclude', 'Exclude', 'exclude', 'Ctrl+Alt+X'],
]

/** Whether the selection is two shapes or more, which booleans combine. */
export const combinable = (editor: Editor) => editor.selection.length > 1 && editor.selected().every((n) => n.kind === 'shape')

/** Combines the selected shapes into the bottommost by `op`. */
export function combine(editor: Editor, op: BooleanOp) {
  if (combinable(editor)) editor.set({ selection: editor.apply({ type: 'boolean', ids: editor.selection, op }) })
}

export type Align = 'left' | 'center' | 'right' | 'top' | 'middle' | 'bottom' | 'distributeX' | 'distributeY' | 'tidy'

/** Title, icon and Alt shortcut of each alignment. */
export const ALIGNS: [Align, string, IconName, string][] = [
  ['left', 'Align left', 'alignLeftEdges', 'Alt+A'],
  ['center', 'Align horizontal centers', 'alignCenters', 'Alt+H'],
  ['right', 'Align right', 'alignRightEdges', 'Alt+D'],
  ['top', 'Align top', 'alignTopEdges', 'Alt+W'],
  ['middle', 'Align vertical centers', 'alignMiddles', 'Alt+V'],
  ['bottom', 'Align bottom', 'alignBottomEdges', 'Alt+S'],
  ['distributeX', 'Distribute horizontal spacing', 'distributeX', 'Alt+Shift+H'],
  ['distributeY', 'Distribute vertical spacing', 'distributeY', 'Alt+Shift+V'],
  ['tidy', 'Tidy up', 'tidy', 'Ctrl+Alt+Shift+T'],
]

/**
 * Aligns `layers` to their bounds, or a single layer to its parent frame or page;
 * distributes and tidies three and two layers or more.
 */
export function align(editor: Editor, how: Align, layers = editor.selected()) {
  const nodes = layers.filter((n) => !n.locked)
  const parent = nodes.length === 1 && editor.nodes.get(nodes[0].id)?.parent
  const to = nodes.length > 1 ? bounds(nodes) : parent && parent.kind === 'frame' ? parent : { x: 0, y: 0, w: editor.page.width, h: editor.page.height }
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
    const gaps = rows.flatMap((row) => row.slice(1).map((n, i) => n.x - row[i].x - row[i].w)).filter((g) => g > 0)
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

/** Layers that follow each other along `axis` without overlapping, each beside the next, and the gaps between them. */
export type Row = { axis: 'x' | 'y'; nodes: Node[]; gaps: number[] }

/** The selected layers as a row, or null when they are not one or an auto layout places them. */
export function rowOf(editor: Editor): Row | null {
  const nodes = editor.selected()
  const parents = new Set(nodes.map((n) => editor.nodes.get(n.id)?.parent))
  const [parent] = parents
  if (nodes.length < 2 || parents.size > 1 || (parent?.kind === 'frame' && parent.direction !== 'none') || nodes.some((n) => n.locked || n.rotation)) return null
  for (const [x, w, y, h] of [['x', 'w', 'y', 'h'], ['y', 'h', 'x', 'w']] as const) {
    const s = [...nodes].sort((a, b) => a[x] - b[x])
    const gaps = s.slice(1).map((n, i) => n[x] - s[i][x] - s[i][w])
    if (gaps.every((g) => g >= 0) && s.slice(1).every((n, i) => n[y] < s[i][y] + s[i][h] && s[i][y] < n[y] + n[h])) return { axis: x, nodes: s, gaps }
  }
  return null
}

/** Moves the layers of `row` after its first so that `gap` lies between each and the next. */
export function setGap(editor: Editor, { axis, nodes }: Row, gap: number) {
  const w = axis === 'x' ? 'w' : 'h'
  let at = nodes[0][axis]
  editor.batch(() => {
    for (const n of nodes) {
      if (n[axis] !== at) editor.apply({ type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w, h: n.h, [axis]: at })
      at += n[w] + gap
    }
  })
}

/** A button for each alignment of the selection, or of the layers `inside` a group, those it cannot do disabled. */
export function AlignBar({ editor, inside }: { editor: Editor; inside?: Node[] }) {
  const n = inside?.length ?? editor.selection.length
  return (
    <div role="toolbar" aria-label={inside ? 'Align inside' : 'Align'} className="align">
      {inside && <span className="align-inside">Inside</span>}
      {ALIGNS.map(([how, title, icon, key]) => (
        <button
          key={how}
          type="button"
          className="icon-button"
          aria-label={title}
          title={inside ? `${title} inside the group` : `${title} (${key})`}
          disabled={n < (how === 'tidy' ? 2 : how.startsWith('distribute') ? 3 : 1)}
          onClick={() => align(editor, how, inside)}
        >
          <Icon name={icon} />
        </button>
      ))}
    </div>
  )
}

/** A button for each boolean of the selected shapes. */
export function BooleanBar({ editor }: { editor: Editor }) {
  return (
    <div role="toolbar" aria-label="Boolean" className="align">
      {BOOLEANS.map(([op, title, icon, key]) => (
        <button key={op} type="button" className="icon-button" aria-label={title} title={`${title} (${key})`} onClick={() => combine(editor, op)}>
          <Icon name={icon} />
        </button>
      ))}
    </div>
  )
}
