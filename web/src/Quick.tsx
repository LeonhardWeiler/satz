import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { AlignBar, BooleanBar, combinable } from './align'
import { Sizing } from './AutoLayout'
import { Chip, NO_SCOPE, Picker } from './ColorPicker'
import { neutral, solid, type Color } from './color'
import { ContextMenu } from './ContextMenu'
import { gradient } from './Paints'
import { Field, Segmented, Select } from './controls'
import { scopeOf, useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { isOpen, type Fill, type Node, type Props, type TextProps } from './model'
import { Popover } from './Popover'
import { ARROWS } from './Properties'
import { ALIGNS, Font, sameOf, Specimen, TypeOptions } from './Text'

type Paint = 'fills' | 'strokes'

/** The most used properties of the selection, above it on the canvas. */
export function Quick({ editor }: { editor: Editor }) {
  useEditor(editor, (e) => e.selection)
  const cropping = useEditor(editor, (e) => e.cropping)
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [menu, setMenu] = useState<{ at: DOMRect; paint: Paint; custom?: boolean } | null>(null)
  const [fitAt, setFitAt] = useState<DOMRect | null>(null)
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
    const mixed = nodes.some((n) => JSON.stringify(n[paint]) !== JSON.stringify(nodes[0][paint]))
    return (
      <button
        type="button"
        className={`icon-button ${paint}`}
        title={title}
        aria-haspopup="menu"
        onClick={(e) => setMenu({ at: e.currentTarget.getBoundingClientRect(), paint })}
      >
        {mixed ? (
          <span className="chip mixed" title="Mixed" />
        ) : p?.type === 'solid' ? (
          <Chip color={p.color} scope={scope} />
        ) : p?.type === 'linear' || p?.type === 'radial' ? (
          <span className="chip" style={{ background: gradient(p, scope) }} />
        ) : (
          p ? <Icon name="image" /> : <span className="chip none" />
        )}
      </button>
    )
  }
  const stroked = nodes.every((n) => n.kind !== 'group')
  const size = sameOf(spans, (a) => a.size)
  return (
    <>
      {nodes.length > 1 && <AlignBar editor={editor} />}
      {combinable(editor) && <BooleanBar editor={editor} />}
      {one?.kind === 'group' && one.children.length > 1 && <AlignBar editor={editor} inside={one.children} />}
      {one && <Sizing editor={editor} node={one} set={set} />}
      {one?.kind === 'text' && (
        <>
          <Specimen editor={editor} spans={spans} format={format} />
          <Font editor={editor} fonts={spans.map((a) => a.font)} set={(font) => format({ font })} />
          <Field label={<Icon name="fontSize" />} title="Font size" unit="pt" min={0.1} value={size} onCommit={(v) => format({ size: v })} />
          <Segmented label="Text align" value={align} options={ALIGNS} onChange={(textAlign) => format({ textAlign })} />
          <TypeOptions spans={spans} set={format} />
        </>
      )}
      {!nodes.some((n) => n.kind === 'group' || n.ppi !== undefined || isOpen(n)) && button('fills', 'Fill color')}
      {stroked && button('strokes', 'Stroke color')}
      {stroked && one && one.strokes.length > 0 && (
        <Field label={<Icon name="strokeWeight" />} title="Stroke weight" unit="pt" value={one.strokeWeight} onCommit={(strokeWeight) => set({ strokeWeight })} />
      )}
      {one && isOpen(one) && one.strokes.length > 0 && (
        <>
          <Select label="Start point" prefix="Start" value={one.arrowStart ? 'arrow' : 'none'} options={ARROWS} onChange={(v) => set({ arrowStart: v === 'arrow' })} />
          <Select label="End point" prefix="End" value={one.arrowEnd ? 'arrow' : 'none'} options={ARROWS} onChange={(v) => set({ arrowEnd: v === 'arrow' })} />
        </>
      )}
      {one && one.ppi !== undefined && (
        <>
          <button
            type="button"
            className="icon-button"
            title="Crop"
            aria-pressed={cropping === one.id}
            onClick={() => editor.set({ cropping: cropping === one.id ? null : one.id })}
          >
            <Icon name="crop" />
          </button>
          <button type="button" className="icon-button" title="Fit image" aria-haspopup="menu" onClick={(e) => setFitAt(e.currentTarget.getBoundingClientRect())}>
            <Icon name="fit" />
          </button>
          <Adjust node={one} set={set} />
          {fitAt &&
            createPortal(
              <ContextMenu
                anchor={() => fitAt}
                side="bottom"
                label="Fit image"
                onClose={() => setFitAt(null)}
                items={FITS.map(([fit, name]) => [name, () => editor.apply({ type: 'fitImage', id: one.id, fit }), true])}
              />,
              document.body,
            )}
        </>
      )}
      {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
        <Field label="Points" title="Points" unit="" int min={3} max={60} value={one.count} onCommit={(count) => set({ count })} />
      )}
      {menu?.custom &&
        createPortal(
          <Picker
            anchor={() => menu.at}
            side="right"
            label={menu.paint === 'fills' ? 'Fill color' : 'Stroke color'}
            color={nodes[0][menu.paint].findLast((f) => f.visible && f.type === 'solid')?.color ?? neutral('black', snapshot.colorMode)}
            mode={snapshot.colorMode}
            scope={scopeOf(snapshot, nodes[0].activeModes)}
            tabs
            onChange={(c) => set({ [menu.paint]: [solid(c)] })}
            onClose={() => setMenu(null)}
          />,
          document.body,
        )}
      {menu && !menu.custom &&
        createPortal(
          <ContextMenu
            anchor={() => menu.at}
            side="bottom"
            label={menu.paint === 'fills' ? 'Fill color' : 'Stroke color'}
            onClose={() => setMenu(null)}
            items={[
              ...colors.map(([name, color, chip]): [ReactNode, () => void, boolean, boolean] => [
                <>
                  {chip ?? (color ? <Chip color={color} scope={NO_SCOPE} /> : <span className="chip none" />)}
                  {name}
                </>,
                () => set({ [menu.paint]: color ? [solid(color)] : [] }),
                true,
                JSON.stringify(current(menu.paint) ?? null) === JSON.stringify(color),
              ]),
              null,
              ['Custom…', () => setMenu({ ...menu, custom: true }), true],
            ]}
          />,
          document.body,
        )}
    </>
  )
}

const FITS = [
  ['cover', 'Fill frame'],
  ['contain', 'Fit in frame'],
  ['frame', 'Frame to image'],
] as const
const ADJUST = ['Brightness', 'Contrast', 'Saturation']

/** The brightness, contrast and saturation of the image fills of `node`, in a popover. */
function Adjust({ node, set }: { node: Node; set: (props: Props) => void }) {
  const [at, setAt] = useState<DOMRect | null>(null)
  const image = node.fills.find((f) => f.type === 'image')!
  const adjust = image.adjust ?? [0, 0, 0]
  const change = (i: number, v: number) => {
    const to = adjust.with(i, v / 100) as Fill['adjust']
    set({ fills: node.fills.map((f) => (f.type === 'image' ? { ...f, adjust: to } : f)) })
  }
  return (
    <>
      <button
        type="button"
        className="icon-button"
        title="Adjust image"
        aria-haspopup="dialog"
        aria-expanded={!!at}
        onClick={(e) => setAt(e.currentTarget.getBoundingClientRect())}
      >
        <Icon name="adjust" />
      </button>
      {at &&
        createPortal(
          <div className="menu-backdrop" onPointerDown={() => setAt(null)}>
            <Popover
              anchor={() => at}
              side="bottom"
              className="picker options"
              role="dialog"
              aria-label="Adjust image"
              tabIndex={-1}
              onPointerDown={(e) => e.stopPropagation()}
              onKeyDown={(e) => {
                if (e.key !== 'Escape') return
                e.stopPropagation()
                setAt(null)
              }}
            >
              <div className="adjust">
                {ADJUST.map((name, i) => (
                  <Field key={name} label={name} unit="%" int min={-100} max={100} reset={0} value={Math.round(adjust[i] * 100)} onCommit={(v) => change(i, v)} />
                ))}
                <button type="button" className="button" disabled={adjust.every((v) => !v)} onClick={() => set({ fills: node.fills.map((f) => (f.type === 'image' ? { ...f, adjust: [0, 0, 0] } : f)) })}>
                  Reset
                </button>
              </div>
            </Popover>
          </div>,
          document.body,
        )}
    </>
  )
}
