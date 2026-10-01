import { Field, Section, Segmented, Select } from './controls'
import type { Editor } from './editor'
import { Icon } from './icons'
import type { Layout, Node, Props, Size } from './model'
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

const SIZES: Record<Size, string> = { fixed: 'Fixed', hug: 'Hug', fill: 'Fill' }
const DIRECTIONS = [
  ['vertical', 'Vertical layout', 'arrowDown'],
  ['horizontal', 'Horizontal layout', 'arrowRight'],
] as const

/** `node` is laid out by the auto layout of its parent. */
export function flows(editor: Editor, node: Node) {
  const parent = editor.nodes.get(node.id)?.parent
  return parent?.kind === 'frame' && parent.direction !== 'none' && !node.absolute
}

/** Width and height sizing of `node`: hug for auto layout frames and text in one, fill in one. */
export function Sizing({ editor, node, set }: { editor: Editor; node: Node; set: (p: Props) => void }) {
  const fills = flows(editor, node)
  const hugs = (node.kind === 'frame' && node.direction !== 'none') || (node.kind === 'text' && fills)
  if (!hugs && !fills) return null
  const sizes = Object.fromEntries(Object.entries(SIZES).filter(([s]) => s === 'fixed' || (s === 'hug' ? hugs : fills))) as Record<Size, string>
  return (
    <div className="grid">
      {(['horizontal', 'vertical'] as const).map((axis) => (
        <Select
          key={axis}
          label={`${axis === 'horizontal' ? 'Width' : 'Height'} sizing`}
          value={node.sizing[axis]}
          options={sizes}
          onChange={(s) => set({ sizing: { ...node.sizing, [axis]: s } })}
        />
      ))}
    </div>
  )
}

/** Direction, alignment, gap and padding of an auto layout frame, or a button that adds one. */
export function AutoLayout({ editor, node, set }: { editor: Editor; node: Node & Layout; set: (p: Props) => void }) {
  if (node.direction === 'none') return <Section title="Auto layout" onAdd={() => editor.apply({ type: 'autoLayout', ids: [node.id] })} />
  const horizontal = node.direction === 'horizontal'
  const between = node.alignMain === 'spaceBetween'
  const length = (prop: 'gap' | (typeof PADDING)[number][0], title: string, label: string) => (
    <Bindable editor={editor} id={node.id} prop={prop} title={`${title} in mm`} label={label}>
      <Field label={label} title={`${title} in mm`} unit="mm" min={prop === 'gap' ? -Infinity : 0} reset={0} value={node[prop]} onCommit={(v) => set({ [prop]: v })} />
    </Bindable>
  )
  return (
    <Section title="Auto layout">
      <div className="row">
        <Segmented label="Direction" value={node.direction} options={DIRECTIONS} onChange={(direction) => set({ direction })} />
        <button
          type="button"
          className="icon-button"
          aria-label="Remove auto layout"
          title="Remove auto layout (Shift+Alt+A)"
          onClick={() => set({ direction: 'none' })}
        >
          <Icon name="minus" />
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
