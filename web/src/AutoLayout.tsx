import { Field, Section, Select } from './controls'
import { MM, type Editor } from './editor'
import { Icon } from './icons'
import type { Layout, Node, Props } from './model'
import { Bindable } from './Variables'

const SPOTS = ['start', 'center', 'end'] as const
const ROWS = ['top', 'center', 'bottom']
const COLUMNS = ['left', 'center', 'right']
const SPACING = { packed: 'Packed', spaceBetween: 'Space between' }
const PADDING = [
  ['paddingTop', 'Top padding', 'T'],
  ['paddingRight', 'Right padding', 'R'],
  ['paddingBottom', 'Bottom padding', 'B'],
  ['paddingLeft', 'Left padding', 'L'],
] as const

/** Direction, alignment, gap and padding of an auto layout frame, or a button that adds one. */
export function AutoLayout({ editor, node, set }: { editor: Editor; node: Node & Layout; set: (p: Props) => void }) {
  if (node.direction === 'none') return <Section title="Auto layout" onAdd={() => editor.apply({ type: 'autoLayout', ids: [node.id] })} />
  const horizontal = node.direction === 'horizontal'
  const between = node.alignMain === 'spaceBetween'
  const length = (prop: 'gap' | (typeof PADDING)[number][0], title: string, label: string) => (
    <Bindable editor={editor} id={node.id} prop={prop} title={`${title} in mm`} label={label}>
      <Field label={label} title={`${title} in mm`} unit="mm" value={node[prop] / MM} onCommit={(v) => set({ [prop]: v * MM })} />
    </Bindable>
  )
  return (
    <Section title="Auto layout">
      <div className="row">
        <div role="radiogroup" aria-label="Direction" className="segmented">
          {(['vertical', 'horizontal'] as const).map((d) => (
            <button
              key={d}
              type="button"
              role="radio"
              aria-checked={node.direction === d}
              aria-label={`${d === 'vertical' ? 'Vertical' : 'Horizontal'} layout`}
              title={`${d === 'vertical' ? 'Vertical' : 'Horizontal'} layout`}
              onClick={() => set({ direction: d })}
            >
              <Icon name={d === 'vertical' ? 'down' : 'right'} size={16} />
            </button>
          ))}
        </div>
        <button
          type="button"
          className="icon-button"
          aria-label="Remove auto layout"
          title="Remove auto layout (Shift+Alt+A)"
          onClick={() => set({ direction: 'none' })}
        >
          <Icon name="minus" size={16} />
        </button>
      </div>
      <div className="auto-layout">
        <div role="radiogroup" aria-label="Alignment" className="align-grid">
          {ROWS.map((row, r) =>
            COLUMNS.map((column, c) => {
              const [main, cross] = horizontal ? [c, r] : [r, c]
              const checked = SPOTS[cross] === node.alignCross && (between ? main === 0 : SPOTS[main] === node.alignMain)
              return (
                <button
                  key={`${row} ${column}`}
                  type="button"
                  role="radio"
                  aria-checked={checked}
                  aria-label={`Align ${row} ${column}`}
                  title={`Align ${row} ${column}`}
                  onClick={() => set({ alignMain: between ? 'spaceBetween' : SPOTS[main], alignCross: SPOTS[cross] })}
                />
              )
            }),
          )}
        </div>
        <div className="auto-layout-gap">
          {length('gap', 'Gap', 'Gap')}
          <Select
            label="Spacing mode"
            value={between ? 'spaceBetween' : 'packed'}
            options={SPACING}
            onChange={(v) => set({ alignMain: v === 'spaceBetween' ? 'spaceBetween' : 'start' })}
          />
        </div>
      </div>
      <div className="grid">{PADDING.map(([prop, title, label]) => length(prop, title, label))}</div>
    </Section>
  )
}
