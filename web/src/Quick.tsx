import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { Chip, NO_SCOPE } from './ColorPicker'
import { ContextMenu } from './ContextMenu'
import { Field, Segmented } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { radiusOf, type Fill, type Props } from './model'
import { ALIGNS, sameOf, Specimen } from './Text'

/** The most used properties of the selection, above it on the canvas. */
export function Quick({ editor }: { editor: Editor }) {
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
          <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => editor.apply({ type: 'format', id: one.id, range: null, textAlign })} />
        </>
      )}
      {!nodes.some((n) => n.kind === 'group' || n.ppi !== undefined) && (
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
        <Field label="R" title="Corner radius in mm" unit="mm" value={radiusOf(one)} onCommit={(radius) => set({ radius, corners: [] })} />
      )}
      {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
        <Field label="N" title="Count" unit="" int value={one.count} onCommit={(v) => set({ count: Math.round(v) })} />
      )}
      {one?.ppi !== undefined && <span className="quick-info">{Math.round(one.ppi)} ppi</span>}
      {menu &&
        createPortal(
          <ContextMenu
            anchor={() => menu}
            side="bottom"
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
