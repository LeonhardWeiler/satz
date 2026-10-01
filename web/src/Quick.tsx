import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { AlignBar } from './align'
import { Sizing } from './AutoLayout'
import { Chip, NO_SCOPE } from './ColorPicker'
import { neutral, solid, type Color } from './color'
import { ContextMenu } from './ContextMenu'
import { gradient } from './Paints'
import { Field, Segmented } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { isOpen, type Props, type TextProps } from './model'
import { ALIGNS, Font, sameOf, Specimen, TypeOptions } from './Text'

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
  const format = (p: TextProps) => editor.apply({ type: 'format', id: one!.id, range: null, ...p })
  /** The colour of the top paint, or a value no colour equals if that is no solid colour. */
  const current = (paint: Paint) => {
    const p = nodes[0][paint].findLast((f) => f.visible)
    return p ? (p.type === 'solid' ? p.color : p.type) : null
  }
  const colors: [string, Color | null, ReactNode?][] = [
    ['None', null],
    ['Black', neutral('black', snapshot.colorMode)],
    ['White', neutral('white', snapshot.colorMode)],
    ...snapshot.swatches.map((s): [string, Color, ReactNode] => [s.name, { swatch: s.id, tint: 1, alpha: 1 }, <Chip color={s.color} scope={NO_SCOPE} />]),
  ]
  const button = (paint: Paint, title: string) => {
    const p = nodes[0][paint].findLast((f) => f.visible)
    const scope = scopeOf(snapshot, nodes[0].activeModes)
    return (
      <button
        type="button"
        className={`icon-button ${paint}`}
        title={title}
        aria-haspopup="menu"
        onClick={(e) => setMenu({ at: e.currentTarget.getBoundingClientRect(), paint })}
      >
        {p?.type === 'solid' ? (
          <Chip color={p.color} scope={scope} />
        ) : p?.type === 'linear' || p?.type === 'radial' ? (
          <span className="chip" style={{ background: gradient(p, scope) }} />
        ) : (
          <Icon name={p ? 'image' : 'minus'} />
        )}
      </button>
    )
  }
  const stroked = nodes.every((n) => n.kind === 'shape' || n.kind === 'frame')
  const size = sameOf(spans, (a) => a.size)
  return (
    <>
      {nodes.length > 1 && <AlignBar editor={editor} />}
      {one && <Sizing editor={editor} node={one} set={set} />}
      {one?.kind === 'text' && (
        <>
          <Specimen editor={editor} spans={spans} format={format} />
          <Font editor={editor} fonts={spans.map((a) => a.font)} set={(font) => format({ font })} />
          <Field label={<Icon name="fontSize" />} title="Font size in pt" unit="pt" min={0.1} value={size} onCommit={(v) => format({ size: v })} />
          <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => format({ textAlign })} />
          <TypeOptions spans={spans} set={format} />
        </>
      )}
      {!nodes.some((n) => n.kind === 'group' || n.ppi !== undefined || isOpen(n)) && button('fills', 'Fill color')}
      {stroked && button('strokes', 'Stroke color')}
      {stroked && one && one.strokes.length > 0 && (
        <Field label={<Icon name="strokeWeight" />} title="Stroke weight" unit="pt" value={one.strokeWeight} onCommit={(strokeWeight) => set({ strokeWeight })} />
      )}
      {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
        <Field label="N" title="Count" unit="" int min={3} max={60} value={one.count} onCommit={(count) => set({ count })} />
      )}
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
