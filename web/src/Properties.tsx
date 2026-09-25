import { neutral, type Color, type ColorMode } from './color'
import { Field, Section, Select } from './controls'
import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { MM, bounds, ends, scopeOf, useEditor, type Editor } from './editor'
import type { Bindable as Prop, Blend, Constraint, Command, Fill, Node, Props, Size, Style } from './model'
import { AutoLayout } from './AutoLayout'
import { EffectList, PaintList } from './Paints'
import { TextFrameSection, TextSection, TextStyles } from './Text'
import { Bindable, ModeSelects, Variables } from './Variables'

const BLENDS: Record<Blend, string> = {
  normal: 'Normal', multiply: 'Multiply', screen: 'Screen', overlay: 'Overlay', darken: 'Darken',
  lighten: 'Lighten', colorDodge: 'Color dodge', colorBurn: 'Color burn', hardLight: 'Hard light',
  softLight: 'Soft light', difference: 'Difference', exclusion: 'Exclusion', hue: 'Hue',
  saturation: 'Saturation', color: 'Color', luminosity: 'Luminosity',
}
const ALIGNS: Record<Style['strokeAlign'], string> = { inside: 'Inside', center: 'Center', outside: 'Outside' }
const JOINS: Record<Style['join'], string> = { miter: 'Miter join', round: 'Round join', bevel: 'Bevel join' }
const CAPS: Record<Style['cap'], string> = { none: 'No cap', round: 'Round cap', square: 'Square cap' }
const ENDS = { none: 'None', arrow: 'Line arrow' } as const
const MODES: Record<ColorMode, string> = { rgb: 'RGB', cmyk: 'CMYK' }
const HORIZONTAL: Record<Constraint, string> = { min: 'Left', max: 'Right', stretch: 'Left & right', center: 'Center', scale: 'Scale' }
const SIZES: Record<Size, string> = { fixed: 'Fixed', hug: 'Hug', fill: 'Fill' }
const VERTICAL: Record<Constraint, string> = { min: 'Top', max: 'Bottom', stretch: 'Top & bottom', center: 'Center', scale: 'Scale' }
const solid = (color: Color): Fill => ({ type: 'solid', color, stops: [], transform: [1, 0, 0, 1, 0, 0], visible: true })

export function Properties({ editor, onExport }: { editor: Editor; onExport: () => void }) {
  const page = useEditor(editor, (e) => e.page)
  const rasterPpi = useEditor(editor, (e) => e.snapshot.rasterPpi)
  const mode = useEditor(editor, (e) => e.snapshot.colorMode)
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const [variables, setVariables] = useState(false)
  const selection = useEditor(editor, (e) => e.selection)
  const nodes = selection.flatMap((id) => editor.nodes.get(id)?.node ?? [])

  const same = <T,>(get: (n: Node) => T | undefined) => {
    const values = nodes.map(get)
    return values.every((v) => v === values[0]) ? (values[0] ?? null) : null
  }
  const each = (cmd: (n: Node) => Command | undefined) => {
    editor.apply({ type: 'beginUndoGroup' })
    try {
      for (const n of nodes) {
        const c = cmd(n)
        if (c) editor.apply(c)
      }
    } finally {
      editor.apply({ type: 'endUndoGroup' })
    }
  }
  const frame = (key: 'x' | 'y' | 'w' | 'h') => (v: number) =>
    each((n) => ({ type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w, h: n.h, [key]: v * MM }))

  const one = nodes.length === 1 ? nodes[0] : undefined
  const scope = scopeOf(snapshot, one?.activeModes)
  const parent = one && editor.nodes.get(one.id)?.parent
  const flows = !!one && parent?.kind === 'frame' && parent.direction !== 'none' && !one.absolute
  const hugs = (one?.kind === 'frame' && one.direction !== 'none') || one?.kind === 'text'
  const sizes = Object.fromEntries(Object.entries(SIZES).filter(([s]) => s === 'fixed' || (s === 'hug' ? hugs : flows))) as Record<Size, string>
  const box = nodes.length ? bounds(nodes) : undefined
  const bindable = (prop: Prop, title: string, label: string, field: ReactNode) =>
    one ? (
      <Bindable editor={editor} id={one.id} prop={prop} title={title} label={label}>
        {field}
      </Bindable>
    ) : (
      field
    )
  const set = (props: Props) => one && editor.apply({ type: 'set', id: one.id, ...props })
  const open = one?.kind === 'shape' && one.shape === 'path' && !one.path.includes(5)
  const line = one && ends(one)
  const length = line && Math.hypot(line[1].x - line[0].x, line[1].y - line[0].y)
  const angle = line && (Math.atan2(line[0].y - line[1].y, line[1].x - line[0].x) * 180) / Math.PI
  const setLine = (length: number, degrees: number) => {
    if (!one || !line) return
    const [a] = line
    const r = (degrees * Math.PI) / 180
    editor.apply({ type: 'setPath', id: one.id, path: [0, a.x, a.y, 1, a.x + length * Math.cos(r), a.y - length * Math.sin(r)] })
  }

  return (
    <aside
      className="panel properties"
      aria-label="Properties"
      onPointerDown={editor.gesture}
    >
      <header className="panel-header">
        <h2>{one ? one.name : nodes.length ? `${nodes.length} layers` : 'Page'}</h2>
        <button type="button" className="primary" onClick={onExport} title="Export PDF (Ctrl+Shift+E)">
          Export PDF
        </button>
      </header>
      {!box && (
        <Section title="Page">
          <div className="grid">
            {(['width', 'height', 'bleed'] as const).map((k) => (
              <Field
                key={k}
                label={k === 'bleed' ? 'Bleed' : k === 'width' ? 'W' : 'H'}
                value={page[k] / MM}
                unit="mm"
                onCommit={(v) => editor.apply({ type: 'setPage', id: page.id, [k]: v * MM })}
              />
            ))}
            <Field
              label="Raster"
              value={rasterPpi}
              unit="ppi"
              onCommit={(v) => editor.apply({ type: 'setDocument', rasterPpi: v })}
            />
            <Select
              label="Color mode"
              value={mode}
              options={MODES}
              onChange={(colorMode) => editor.apply({ type: 'setDocument', colorMode })}
            />
            <ModeSelects editor={editor} id={page.id} own={page.modes} inherited={{}} />
          </div>
        </Section>
      )}
      {!box && (
        <Section title="Variables">
          <button type="button" className="button" onClick={() => setVariables(true)}>
            Local variables
          </button>
          {variables && createPortal(<Variables editor={editor} onClose={() => setVariables(false)} />, document.body)}
        </Section>
      )}
      {!box && <TextStyles editor={editor} />}
      {box && (
        <Section title="Layout">
          <div className="grid">
            <Field label="X" value={nodes.length > 1 ? same((n) => n.x / MM) : box.x / MM} unit="mm" onCommit={frame('x')} />
            <Field label="Y" value={nodes.length > 1 ? same((n) => n.y / MM) : box.y / MM} unit="mm" onCommit={frame('y')} />
            {line ? (
              <>
                <Field label="L" title="Length in mm" value={length! / MM} unit="mm" onCommit={(v) => setLine(Math.max(0, v * MM), angle!)} />
                <Field label="∠" title="Angle in °" value={angle!} unit="°" onCommit={(v) => setLine(length!, v)} />
              </>
            ) : (
              <>
                {bindable('w', 'W in mm', 'W', <Field label="W" value={same((n) => n.w / MM)} unit="mm" onCommit={frame('w')} />)}
                {bindable('h', 'H in mm', 'H', <Field label="H" value={same((n) => n.h / MM)} unit="mm" onCommit={frame('h')} />)}
              </>
            )}
            {one?.kind === 'shape' && one.shape === 'rect' && (
              bindable(
                'radius',
                'Corner radius in mm',
                'R',
                <Field label="R" title="Corner radius in mm" unit="mm" value={one.radius / MM} onCommit={(v) => set({ radius: v * MM })} />,
              )
            )}
            {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
              <Field label="N" title="Count" unit="" value={one.count} onCommit={(v) => set({ count: Math.round(v) })} />
            )}
            {one?.kind === 'shape' && one.shape === 'star' && (
              <Field label="Ratio" title="Star ratio in %" unit="%" value={one.ratio * 100} onCommit={(v) => set({ ratio: v / 100 })} />
            )}
          </div>
          {one && (hugs || flows) && (
            <div className="grid">
              {(['horizontal', 'vertical'] as const).map((axis) => (
                <Select
                  key={axis}
                  label={`${axis === 'horizontal' ? 'Width' : 'Height'} sizing`}
                  value={one.sizing[axis]}
                  options={sizes}
                  onChange={(s) => set({ sizing: { ...one.sizing, [axis]: s } })}
                />
              ))}
            </div>
          )}
          {one && parent?.kind === 'frame' && parent.direction !== 'none' && (
            <label className="check">
              <input type="checkbox" checked={one.absolute} onChange={(e) => set({ absolute: e.currentTarget.checked })} />
              Absolute position
            </label>
          )}
          {one && parent?.kind === 'frame' && !flows && (
            <div className="grid">
              {(['horizontal', 'vertical'] as const).map((axis) => (
                <Select
                  key={axis}
                  label={`${axis === 'horizontal' ? 'Horizontal' : 'Vertical'} constraint`}
                  value={one.constraints[axis]}
                  options={axis === 'horizontal' ? HORIZONTAL : VERTICAL}
                  onChange={(k) => set({ constraints: { ...one.constraints, [axis]: k } })}
                />
              ))}
            </div>
          )}
          {one?.kind === 'frame' && (
            <label className="check">
              <input
                type="checkbox"
                checked={one.clip}
                onChange={(e) => editor.apply({ type: 'set', id: one.id, clip: e.currentTarget.checked })}
              />
              Clip content
            </label>
          )}
        </Section>
      )}
      {one?.kind === 'frame' && <AutoLayout editor={editor} node={one} set={set} />}
      {one && (
        <Section title="Layer">
          <div className="grid">
            {bindable(
              'opacity',
              'Opacity',
              '',
              <Field label="" title="Opacity" unit="%" value={one.opacity * 100} onCommit={(v) => set({ opacity: v / 100 })} />,
            )}
            <Select label="Blend mode" value={one.blend} options={BLENDS} onChange={(blend) => set({ blend })} />
            {one.kind === 'frame' && (
              <ModeSelects editor={editor} id={one.id} own={one.modes} inherited={editor.nodes.get(one.id)?.parent?.activeModes ?? page.modes} />
            )}
          </div>
          <label className="check">
            <input type="checkbox" checked={one.mask} onChange={(e) => set({ mask: e.currentTarget.checked })} />
            Use as mask
          </label>
        </Section>
      )}
      {nodes.length > 1 && (
        <Section title="Layer">
          <button
            type="button"
            className="button"
            title="Use as mask (Ctrl+Alt+M)"
            onClick={() => editor.set({ selection: editor.apply({ type: 'mask', ids: selection }) })}
          >
            Use as mask
          </button>
        </Section>
      )}
      {one && one.kind !== 'group' && (
        <PaintList
          title="Fill"
          paints={one.fills}
          added={solid(neutral(one.kind === 'text' ? 'black' : 'gray', mode))}
          mode={mode}
          scope={scope}
          onChange={(fills) => set({ fills })} />
      )}
      {one && (one.kind === 'shape' || one.kind === 'frame') && (
        <PaintList title="Stroke" paints={one.strokes} added={solid(neutral('black', mode))} mode={mode} scope={scope} onChange={(strokes) => set({ strokes })}>
          {one.strokes.length > 0 && (
            <div className="grid">
              {bindable(
                'strokeWeight',
                'Stroke weight',
                '',
                <Field label="" title="Stroke weight" unit="pt" value={one.strokeWeight} onCommit={(v) => set({ strokeWeight: v })} />,
              )}
              {!open && <Select label="Stroke position" value={one.strokeAlign} options={ALIGNS} onChange={(strokeAlign) => set({ strokeAlign })} />}
              <Select label="Stroke join" value={one.join} options={JOINS} onChange={(join) => set({ join })} />
              {open && (
                <>
                  <Select label="Stroke cap" value={one.cap} options={CAPS} onChange={(cap) => set({ cap })} />
                  <Select
                    label="Start point"
                    value={one.arrowStart ? 'arrow' : 'none'}
                    options={ENDS}
                    onChange={(v) => set({ arrowStart: v === 'arrow' })}
                  />
                  <Select
                    label="End point"
                    value={one.arrowEnd ? 'arrow' : 'none'}
                    options={ENDS}
                    onChange={(v) => set({ arrowEnd: v === 'arrow' })}
                  />
                </>
              )}
            </div>
          )}
        </PaintList>
      )}
      {one && <EffectList effects={one.effects} mode={mode} scope={scope} onChange={(effects) => set({ effects })} />}
      {one?.kind === 'text' && <TextSection editor={editor} node={one} />}
      {one?.kind === 'text' && <TextFrameSection node={one} set={set} />}
    </aside>
  )
}
