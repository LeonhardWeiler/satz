import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { Chip, NO_SCOPE } from './ColorPicker'
import { neutral, solid, type Color } from './color'
import { ContextMenu } from './ContextMenu'
import { Field, Segmented } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { isOpen, radiusOf, type Props } from './model'
import { ALIGNS, sameOf, Specimen } from './Text'

type Paint = 'fills' | 'strokes'

/** The most used properties of the selection, above it on the canvas. */
export function Quick({ editor }: { editor: Editor }) {
  useEditor(editor, (e) => e.selection)
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [menu, setMenu] = useState<{ at: DOMRect; paint: Paint } | null>(null)
  const nodes = editor.selected()
  if (nodes.length === 0) return null
  const one = nodes.length === 1 ? nodes[0] : undefined
  const set = (props: Props) => editor.batch(() => nodes.forEach((n) => editor.apply({ type: 'set', id: n.id, ...props })))
  const spans = one?.kind === 'text' ? (snapshot.stories[one.story]?.spans ?? []) : []
  const align = sameOf(spans, (a) => a.textAlign)
  const current = (paint: Paint) => nodes[0][paint].find((f) => f.visible && f.type === 'solid')?.color
  const colors: [string, Color | null, ReactNode?][] = [
    ['None', null],
    ['Black', neutral('black', snapshot.colorMode)],
    ['White', neutral('white', snapshot.colorMode)],
    ...snapshot.swatches.map((s): [string, Color, ReactNode] => [s.name, { swatch: s.id, tint: 1, alpha: 1 }, <Chip color={s.color} scope={NO_SCOPE} />]),
  ]
  const button = (paint: Paint, title: string) => {
    const color = current(paint)
    return (
      <button
        type="button"
        className={`icon-button ${paint}`}
        title={title}
        aria-haspopup="menu"
        onClick={(e) => setMenu({ at: e.currentTarget.getBoundingClientRect(), paint })}
      >
        {color ? <Chip color={color} scope={scopeOf(snapshot, nodes[0].activeModes)} /> : <Icon name="minus" />}
      </button>
    )
  }
  const stroked = nodes.every((n) => n.kind === 'shape' || n.kind === 'frame')
  return (
    <>
      {one?.kind === 'text' && (
        <>
          <Specimen editor={editor} spans={spans} format={(p) => editor.apply({ type: 'format', id: one.id, range: null, ...p })} />
          <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => editor.apply({ type: 'format', id: one.id, range: null, textAlign })} />
        </>
      )}
      {!nodes.some((n) => n.kind === 'group' || n.ppi !== undefined || isOpen(n)) && button('fills', 'Fill color')}
      {stroked && button('strokes', 'Stroke color')}
      {stroked && one && one.strokes.length > 0 && (
        <Field label={<Icon name="strokeWeight" />} title="Stroke weight" unit="pt" value={one.strokeWeight} onCommit={(strokeWeight) => set({ strokeWeight })} />
      )}
      {one?.kind === 'shape' && one.shape === 'rect' && (
        <Field label="R" title="Corner radius in mm" unit="mm" value={radiusOf(one)} onCommit={(radius) => set({ radius, corners: [] })} />
      )}
      {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
        <Field label="N" title="Count" unit="" int min={3} max={60} value={one.count} onCommit={(count) => set({ count })} />
      )}
      {one?.ppi !== undefined && <span className="quick-info">{Math.round(one.ppi)} ppi</span>}
      {menu &&
        createPortal(
          <ContextMenu
            anchor={() => menu.at}
            side="bottom"
            label={menu.paint === 'fills' ? 'Fill color' : 'Stroke color'}
            onClose={() => setMenu(null)}
            items={colors.map(([name, color, chip]) => [
              <>
                {chip ?? (color ? <Chip color={color} scope={NO_SCOPE} /> : <span className="chip none" />)}
                {name}
              </>,
              () => set({ [menu.paint]: color ? [solid(color)] : [] }),
              true,
              JSON.stringify(current(menu.paint) ?? null) === JSON.stringify(color),
            ])}
          />,
          document.body,
        )}
    </>
  )
}
