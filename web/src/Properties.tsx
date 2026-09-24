import { Field, Section, Select } from './controls'
import { MM, bounds, ends, useEditor, type Editor } from './editor'
import type { Blend, Command, Fill, Node, Props, Style } from './model'
import { EffectList, PaintList } from './Paints'

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
const BLACK: Fill = { type: 'solid', color: 0x000000ff, stops: [], transform: [1, 0, 0, 1, 0, 0], visible: true }
const GRAY: Fill = { ...BLACK, color: 0xd9d9d9ff }

export function Properties({ editor, onExport }: { editor: Editor; onExport: () => void }) {
  const page = useEditor(editor, (e) => e.page)
  const rasterPpi = useEditor(editor, (e) => e.snapshot.rasterPpi)
  const selection = useEditor(editor, (e) => e.selection)
  const nodes = selection.flatMap((id) => editor.nodes.get(id)?.node ?? [])

  const same = <T,>(get: (n: Node) => T | undefined) => {
    const values = nodes.map(get)
    return values.every((v) => v === values[0]) ? (values[0] ?? null) : null
  }
  const each = (cmd: (n: Node) => Command | undefined) => {
    editor.apply({ type: 'beginUndoGroup' })
    for (const n of nodes) {
      const c = cmd(n)
      if (c) editor.apply(c)
    }
    editor.apply({ type: 'endUndoGroup' })
  }
  const frame = (key: 'x' | 'y' | 'w' | 'h') => (v: number) =>
    each((n) => ({ type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w, h: n.h, [key]: Math.max(key === 'w' || key === 'h' ? 0 : -Infinity, v * MM) }))

  const one = nodes.length === 1 ? nodes[0] : undefined
  const box = nodes.length ? bounds(nodes) : undefined
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
    <aside className="panel properties" aria-label="Properties">
      <header className="panel-header">
        <h2>{one ? one.name : nodes.length ? `${nodes.length} layers` : 'Page'}</h2>
        <button type="button" className="primary" onClick={onExport} title="Export PDF (Ctrl+Shift+E)">
          Export PDF
        </button>
      </header>
      {!box && (
        <Section title="Page">
          <div className="grid">
            <Field label="W" value={page.width / MM} unit="mm" readOnly />
            <Field label="H" value={page.height / MM} unit="mm" readOnly />
            <Field label="Bleed" value={page.bleed / MM} unit="mm" readOnly />
            <Field
              label="Raster"
              value={rasterPpi}
              unit="ppi"
              onCommit={(v) => v > 0 && editor.apply({ type: 'setDocument', rasterPpi: v })}
            />
          </div>
        </Section>
      )}
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
                <Field label="W" value={same((n) => n.w / MM)} unit="mm" onCommit={frame('w')} />
                <Field label="H" value={same((n) => n.h / MM)} unit="mm" onCommit={frame('h')} />
              </>
            )}
            {one?.kind === 'shape' && one.shape === 'rect' && (
              <Field label="R" title="Corner radius in mm" unit="mm" value={one.radius / MM} onCommit={(v) => set({ radius: Math.max(0, v * MM) })} />
            )}
            {one?.kind === 'shape' && (one.shape === 'polygon' || one.shape === 'star') && (
              <Field label="N" title="Count" unit="" value={one.count} onCommit={(v) => v >= 3 && set({ count: Math.round(v) })} />
            )}
            {one?.kind === 'shape' && one.shape === 'star' && (
              <Field label="Ratio" title="Star ratio in %" unit="%" value={one.ratio * 100} onCommit={(v) => set({ ratio: Math.min(1, Math.max(0.01, v / 100)) })} />
            )}
          </div>
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
      {one && (
        <Section title="Layer">
          <div className="grid">
            <Field
              label=""
              title="Opacity"
              unit="%"
              value={one.opacity * 100}
              onCommit={(v) => set({ opacity: Math.min(1, Math.max(0, v / 100)) })}
            />
            <Select label="Blend mode" value={one.blend} options={BLENDS} onChange={(blend) => set({ blend })} />
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
        <PaintList title="Fill" paints={one.fills} added={one.kind === 'text' ? BLACK : GRAY} onChange={(fills) => set({ fills })} />
      )}
      {one && (one.kind === 'shape' || one.kind === 'frame') && (
        <PaintList title="Stroke" paints={one.strokes} added={BLACK} onChange={(strokes) => set({ strokes })}>
          {one.strokes.length > 0 && (
            <div className="grid">
              <Field label="" title="Stroke weight" unit="pt" value={one.strokeWeight} onCommit={(v) => v >= 0 && set({ strokeWeight: v })} />
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
      {one && <EffectList effects={one.effects} onChange={(effects) => set({ effects })} />}
      {one?.kind === 'text' && (
        <Section title="Text">
          <div className="grid">
            <Field
              label="Size"
              value={one.size}
              unit="pt"
              onCommit={(size) => size > 0 && editor.apply({ type: 'set', id: one.id, size })}
            />
          </div>
          <textarea
            className="content"
            aria-label="Text content"
            rows={6}
            defaultValue={one.text}
            key={one.id}
            onBlur={(e) => {
              if (e.currentTarget.value !== one.text) editor.apply({ type: 'setText', id: one.id, text: e.currentTarget.value })
            }}
          />
        </Section>
      )}
    </aside>
  )
}
