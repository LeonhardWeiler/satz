import { bounds, MM, type Editor } from './editor'
import type { IconName } from './icons'
import type { Node } from './model'

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
 * Aligns the selected layers to their bounds, or a single layer to its parent frame or
 * page; distributes and tidies three and two layers or more.
 */
export function align(editor: Editor, how: Align) {
  const nodes = editor.selected().filter((n) => !n.locked)
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
