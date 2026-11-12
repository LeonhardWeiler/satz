import { Field, Section } from './controls'
import type { Editor } from './editor'
import type { Container, Node, TextNode } from './model'

type Cell = TextNode & { cell: [number, number, number, number] }

/** The cells of the table `t`, row by row. */
const cellsOf = (t: Container) =>
  t.children.filter((c): c is Cell => c.kind === 'text' && !!c.cell).toSorted((a, b) => a.cell[1] - b.cell[1] || a.cell[0] - b.cell[0])

/** The table that `nodes` are or are cells of, and the cells among them. */
function tableOf(editor: Editor, nodes: Node[]) {
  const tables = new Set(nodes.map((n) => (n.kind === 'frame' && n.table ? n : n.kind === 'text' && n.cell ? editor.nodes.get(n.id)?.parent : undefined)))
  const [t] = tables
  if (tables.size !== 1 || !t?.table) return undefined
  return { t, table: t.table, cells: nodes.filter((n): n is Cell => n.kind === 'text' && !!n.cell) }
}

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

export function TableSection({ editor, nodes }: { editor: Editor; nodes: Node[] }) {
  const of = tableOf(editor, nodes)
  if (!of) return null
  const { t, table, cells } = of
  const span = (a: 0 | 1) => {
    if (!cells.length) return [0, (a ? table.rows : table.columns).length]
    return [Math.min(...cells.map((c) => c.cell[a])), Math.max(...cells.map((c) => c.cell[a] + c.cell[a + 2]))]
  }
  const [c0, c1] = span(0)
  const [r0, r1] = span(1)
  const insert = (column: boolean, at: number) => editor.apply({ type: 'tableInsert', id: t.id, column, at })
  const remove = (column: boolean, from: number, to: number) =>
    editor.batch(() => {
      for (let i = to - 1; i >= from; i--) editor.apply({ type: 'tableDelete', id: t.id, column, at: i })
    })
  const same = (sizes: number[], from: number, to: number) => (sizes.slice(from, to).every((s) => Math.abs(s - sizes[from]) < 1e-6) ? sizes[from] : null)
  const size = (column: boolean, from: number, to: number, v: number) => {
    const sizes = (column ? table.columns : table.rows).map((s, i) => (i >= from && i < to ? v : s))
    editor.apply({ type: 'sizeTable', id: t.id, columns: column ? sizes : table.columns, rows: column ? table.rows : sizes })
  }
  const button = (label: string, onClick: () => void) => (
    <button type="button" className="button" onClick={onClick}>
      {label}
    </button>
  )
  return (
    <Section title="Table">
      <div className="grid">
        <Field label="W" title="Column width" unit="length" min={0.01} value={same(table.columns, c0, c1)} onCommit={(v) => size(true, c0, c1, v)} />
        <Field label="H" title="Row height" unit="length" value={same(table.rows, r0, r1)} onCommit={(v) => size(false, r0, r1, v)} />
        {button('Row above', () => insert(false, r0))}
        {button('Row below', () => insert(false, r1))}
        {button('Column left', () => insert(true, c0))}
        {button('Column right', () => insert(true, c1))}
        {cells.length > 0 && button('Delete rows', () => remove(false, r0, r1))}
        {cells.length > 0 && button('Delete columns', () => remove(true, c0, c1))}
        {cells.length > 1 && button('Merge cells', () => editor.apply({ type: 'mergeCells', ids: cells.map((c) => c.id) }))}
        {cells.length === 1 && (cells[0].cell[2] > 1 || cells[0].cell[3] > 1) && button('Split cell', () => editor.apply({ type: 'splitCell', id: cells[0].id }))}
      </div>
    </Section>
  )
}
