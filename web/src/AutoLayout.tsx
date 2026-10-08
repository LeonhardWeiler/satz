import { Field, Section, Segmented, Select, SIDE_NAMES, Sides } from './controls'
import type { Editor } from './editor'
import { Icon } from './icons'
import type { Layout, Node, Props, Size } from './model'
import { Bindable } from './Variables'

const SPOTS = ['start', 'center', 'end'] as const
const ROWS = ['top', 'center', 'bottom']
const COLUMNS = ['left', 'center', 'right']
const STEPS: Record<string, [number, number]> = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1] }
const SPACING = { packed: 'Fixed', spaceBetween: 'Space between' }
const PADDING = ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft'] as const

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
          prefix={axis === 'horizontal' ? 'W' : 'H'}
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
  const align = (r: number, c: number) => {
    const [main, cross] = horizontal ? [c, r] : [r, c]
    set({ alignMain: between ? 'spaceBetween' : SPOTS[main], alignCross: SPOTS[cross] })
  }
  return (
    <Section
      title="Auto layout"
      actions={
        <button type="button" className="icon-button" aria-label="Remove auto layout" title="Remove auto layout (Shift+Alt+A)" onClick={() => set({ direction: 'none' })}>
          <Icon name="minus" />
        </button>
      }
    >
      <Segmented label="Direction" prefix="Direction" value={node.direction} options={DIRECTIONS} onChange={(direction) => set({ direction })} />
      <div className="auto-layout">
        <div
          role="radiogroup"
          aria-label="Alignment"
          className="align-grid"
          onKeyDown={(e) => {
            const [dr, dc] = STEPS[e.key] ?? []
            if (dr === undefined) return
            e.preventDefault()
            const cells = [...e.currentTarget.children] as HTMLElement[]
            const i = Math.max(cells.indexOf(document.activeElement as HTMLElement), 0)
            const r = Math.min(Math.max(Math.floor(i / 3) + dr, 0), 2)
            const c = Math.min(Math.max((i % 3) + dc, 0), 2)
            align(r, c)
            cells[r * 3 + c].focus()
          }}
        >
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
                  tabIndex={checked ? 0 : -1}
                  onClick={() => align(r, c)}
                />
              )
            }),
          )}
        </div>
        <div className="auto-layout-gap">
          <Bindable editor={editor} id={node.id} prop="gap" title="Gap" label="Gap">
            <Field label="Gap" unit="length" min={-Infinity} reset={0} value={node.gap} onCommit={(gap) => set({ gap })} />
          </Bindable>
          <Select
            label="Spacing mode"
            prefix="Spacing"
            value={between ? 'spaceBetween' : 'packed'}
            options={SPACING}
            onChange={(v) => set({ alignMain: v === 'spaceBetween' ? 'spaceBetween' : 'start' })}
          />
        </div>
      </div>
      <Sides
        key={node.id}
        what="padding"
        values={PADDING.map((p) => node[p])}
        opened={PADDING.some((p) => node.bindings[p])}
        wrap={(i, field) => (
          <Bindable editor={editor} id={node.id} prop={PADDING[i]} title={`${SIDE_NAMES[i]} padding`} label={SIDE_NAMES[i]}>
            {field}
          </Bindable>
        )}
        set={(v) => set(Object.fromEntries(PADDING.map((p, i) => [p, Math.max(0, v[i])])))}
      />
    </Section>
  )
}
