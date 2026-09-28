import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { Chip, NO_SCOPE } from './ColorPicker'
import { ContextMenu } from './ContextMenu'
import { Field } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import type { Fill, Props } from './model'
import { ALIGNS, sameOf, Specimen } from './Text'

/** The most used properties of the selection, above it on the canvas. */
export function Quick({ editor, onMore }: { editor: Editor; onMore: () => void }) {
  useEditor(editor, (e) => e.selection)
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [menu, setMenu] = useState<DOMRect | null>(null)
  const nodes = editor.selected()
  if (nodes.length === 0) return null
  const one = nodes.length === 1 ? nodes[0] : undefined
  const set = (props: Props) => one && editor.apply({ type: 'set', id: one.id, ...props })
  const fill = nodes[0].fills.find((f) => f.visible && f.type === 'solid')
  const spans = one?.kind === 'text' ? (snapshot.stories[one.story]?.spans ?? []) : []
  const align = sameOf(spans, (a) => a.textAlign)
  const pick = (id: string) => {
    const fill: Fill = { type: 'solid', color: { swatch: id, tint: 1, alpha: 1 }, stops: [], transform: [1, 0, 0, 1, 0, 0], visible: true }
    editor.batch(() => nodes.forEach((n) => editor.apply({ type: 'set', id: n.id, fills: [fill] })))
  }
  return (
    <>
      {one?.kind === 'text' && (
        <>
          <Specimen
            editor={editor}
            style={sameOf(spans, (a) => a.textStyle)}
            onPick={(textStyle) => editor.apply({ type: 'format', id: one.id, range: null, textStyle })}
          />
          <div role="radiogroup" aria-label="Text align" className="segmented">
            {ALIGNS.map(([value, title, icon]) => (
              <button
                key={value}
                type="button"
                role="radio"
                aria-checked={align === value}
                aria-label={title}
                title={title}
                onClick={() => editor.apply({ type: 'format', id: one.id, range: null, textAlign: value })}
              >
                <Icon name={icon} />
              </button>
            ))}
          </div>
        </>
      )}
      {!nodes.some((n) => n.kind === 'group') && (
        <button
          type="button"
          className="icon-button"
          title="Fill swatch"
          aria-haspopup="menu"
          onClick={(e) => setMenu(e.currentTarget.getBoundingClientRect())}
        >
          {fill ? <Chip color={fill.color} scope={scopeOf(snapshot, nodes[0].activeModes)} /> : <Icon name="minus" />}
        </button>
      )}
      {one?.kind === 'shape' && one.shape === 'rect' && (
        <Field label="R" title="Corner radius in mm" unit="mm" value={one.radius} onCommit={(radius) => set({ radius })} />
      )}
      {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
        <Field label="N" title="Count" unit="" int value={one.count} onCommit={(v) => set({ count: Math.round(v) })} />
      )}
      {one?.ppi !== undefined && <span className="quick-info">{Math.round(one.ppi)} ppi</span>}
      {one && one.kind !== 'text' && (
        <Field label="" title="Opacity" unit="%" value={one.opacity * 100} onCommit={(v) => set({ opacity: v / 100 })} />
      )}
      <button type="button" className="button" onClick={onMore}>
        More
      </button>
      {menu &&
        createPortal(
          <ContextMenu
            menu={{ x: menu.left, y: menu.bottom + 4 }}
            label="Swatches"
            onClose={() => setMenu(null)}
            items={snapshot.swatches.length ? snapshot.swatches.map((s): [ReactNode, () => void, boolean, boolean] => [
              <>
                <Chip color={s.color} scope={NO_SCOPE} />
                {s.name}
              </>,
              () => pick(s.id),
              true,
              !!fill && typeof fill.color === 'object' && 'swatch' in fill.color && fill.color.swatch === s.id,
            ]) : [['No swatches', () => {}, false]]}
          />,
          document.body,
        )}
    </>
  )
}
