import { spanOf, picked, TABLE_ACTIONS } from './cells'
import type { Action } from './commands'
import { Field, MenuButton, Section } from './controls'
import type { Editor } from './editor'
import type { Node } from './model'

export function TableSection({ editor, nodes }: { editor: Editor; nodes: Node[] }) {
  const s = spanOf(editor, nodes)
  if (!s) return null
  const { t, table, c0, c1, r0, r1 } = s
  const same = (sizes: number[], from: number, to: number) => (sizes.slice(from, to).every((v) => Math.abs(v - sizes[from]) < 1e-6) ? sizes[from] : null)
  const size = (column: boolean, from: number, to: number, v: number) => {
    const sizes = (column ? table.columns : table.rows).map((x, i) => (i >= from && i < to ? v : x))
    editor.apply({ type: 'sizeTable', id: t.id, columns: column ? sizes : table.columns, rows: column ? table.rows : sizes })
  }
  const items = (list: Action[]) => list.map((a): [string, () => void, boolean] => [a.title, () => a.run!(editor), a.can!(editor)])
  return (
    <Section
      title="Table"
      actions={
        <>
          <MenuButton icon="plus" title="Insert rows or columns" items={items(TABLE_ACTIONS.slice(0, 4))} />
          {picked(s) && <MenuButton icon="more" title="Cell options" items={items(TABLE_ACTIONS.slice(4))} />}
        </>
      }
    >
      <div className="grid">
        <Field label="Column" title="Column width" unit="length" min={0.01} value={same(table.columns, c0, c1)} onCommit={(v) => size(true, c0, c1, v)} />
        <Field label="Row" title="Row height" unit="length" value={same(table.rows, r0, r1)} onCommit={(v) => size(false, r0, r1, v)} />
      </div>
    </Section>
  )
}
