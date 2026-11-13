import type { Action } from './commands'
import type { Editor } from './editor'
import type { Container, Node, TextNode } from './model'

type Cell = TextNode & { cell: [number, number, number, number] }

/** The cells of the table `t`, row by row. */
const cellsOf = (t: Container) =>
  t.children.filter((c): c is Cell => c.kind === 'text' && !!c.cell).toSorted((a, b) => a.cell[1] - b.cell[1] || a.cell[0] - b.cell[0])

/** The table that `n` is or is a cell of. */
const holder = (editor: Editor, n: Node) => (n.kind === 'frame' && n.table ? n : n.kind === 'text' && n.cell ? editor.nodes.get(n.id)?.parent : undefined)

/** The table that `nodes` are or are cells of, and the cells among them. */
function tableOf(editor: Editor, nodes: Node[]) {
  const tables = new Set(nodes.map((n) => holder(editor, n)))
  const [t] = tables
  if (tables.size !== 1 || !t?.table) return undefined
  return { t, table: t.table, cells: nodes.filter((n): n is Cell => n.kind === 'text' && !!n.cell) }
}

/** The cells of the tables that the selected layers are or are in. */
export const selectedCells = (editor: Editor) =>
  [...new Set(editor.selected().map((n) => holder(editor, n)))].flatMap((t) => (t?.table ? cellsOf(t) : []))

/** Edits the cell after or, with `back`, before the cell `id`; past the last one it adds a row. */
export function tab(editor: Editor, id: string, back: boolean) {
  const t = editor.nodes.get(id)?.parent
  const n = editor.nodes.get(id)?.node
  if (!t?.table || n?.kind !== 'text' || !n.cell) return
  const at = cellsOf(t).findIndex((c) => c.id === id) + (back ? -1 : 1)
  if (at < 0) return
  if (at === cellsOf(t).length) editor.apply({ type: 'tableInsert', id: t.id, column: false, at: t.table.rows.length })
  const fresh = editor.nodes.get(t.id)?.node
  const to = fresh && 'children' in fresh ? cellsOf(fresh)[at] : undefined
  if (to) editor.set({ selection: [to.id], editing: { id: to.id, anchor: 0, focus: editor.storyOf(to).text.length } })
}

/** The table of the selected layers, their cells and the columns and rows they span, all of them for the table itself. */
export function spanOf(editor: Editor, nodes: Node[]) {
  const of = tableOf(editor, nodes)
  if (!of) return undefined
  const { table, cells } = of
  const span = (a: 0 | 1) =>
    cells.length ? [Math.min(...cells.map((c) => c.cell[a])), Math.max(...cells.map((c) => c.cell[a] + c.cell[a + 2]))] : [0, (a ? table.rows : table.columns).length]
  const [c0, c1] = span(0)
  const [r0, r1] = span(1)
  return { ...of, c0, c1, r0, r1 }
}
type Span = NonNullable<ReturnType<typeof spanOf>>

const act = (title: string, run: (editor: Editor, s: Span) => void, can: (s: Span) => boolean = () => true): Action => ({
  title,
  keys: '',
  group: 'Table',
  run: (editor) => {
    const s = spanOf(editor, editor.selected())
    if (s) run(editor, s)
  },
  can: (editor) => {
    const s = spanOf(editor, editor.selected())
    return !!s && can(s)
  },
})
const insert = (column: boolean, at: (s: Span) => number) => (editor: Editor, s: Span) => editor.apply({ type: 'tableInsert', id: s.t.id, column, at: at(s) })
const remove = (column: boolean) => (editor: Editor, s: Span) =>
  editor.batch(() => {
    for (let i = (column ? s.c1 : s.r1) - 1; i >= (column ? s.c0 : s.r0); i--) editor.apply({ type: 'tableDelete', id: s.t.id, column, at: i })
  })
export const picked = (s: Span) => s.cells.length > 0

/** What the table section's menus and the context menu do to the selected cells or table. */
export const TABLE_ACTIONS = [
  act('Insert row above', insert(false, (s) => s.r0)),
  act('Insert row below', insert(false, (s) => s.r1)),
  act('Insert column left', insert(true, (s) => s.c0)),
  act('Insert column right', insert(true, (s) => s.c1)),
  act('Merge cells', (editor, s) => editor.apply({ type: 'mergeCells', ids: s.cells.map((c) => c.id) }), (s) => s.cells.length > 1),
  act('Split cell', (editor, s) => editor.apply({ type: 'splitCell', id: s.cells[0].id }), (s) => s.cells.length === 1 && (s.cells[0].cell[2] > 1 || s.cells[0].cell[3] > 1)),
  act('Delete rows', remove(false), picked),
  act('Delete columns', remove(true), picked),
]
