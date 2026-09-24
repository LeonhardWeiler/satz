import type { ReactNode } from 'react'
import { Field, RowActions, Section, Select, Swatch, alpha, hex, withAlpha } from './controls'
import { MM } from './editor'
import type { Effect, Fill } from './model'

const TYPES = { solid: 'Solid', linear: 'Linear', radial: 'Radial' } as const
const EFFECTS = { dropShadow: 'Drop shadow', blur: 'Layer blur' } as const
const RADIAL = [0.5, 0, 0, 0.5, 0.5, 0.5]
const SHADOW: Effect = { type: 'dropShadow', x: 0, y: 4, radius: 4, color: 0x00000040, visible: true }
const BLUR: Effect = { type: 'blur', x: 0, y: 0, radius: 4, color: 0, visible: true }

/** Linear gradient across the unit box through its centre, 0° left to right, 90° top to bottom. */
function linear(degrees: number) {
  const a = (degrees * Math.PI) / 180
  const [dx, dy] = [Math.cos(a), Math.sin(a)]
  return [dx, dy, -dy, dx, 0.5 - dx / 2, 0.5 - dy / 2]
}

const angle = (t: number[]) => (Math.atan2(t[1], t[0]) * 180) / Math.PI

function retype(p: Fill, type: Fill['type']): Fill {
  if (type === p.type) return p
  if (type === 'solid') return { ...p, type, color: p.stops[0]?.color ?? p.color }
  const stops = p.stops.length > 1 ? p.stops : [{ at: 0, color: p.color }, { at: 1, color: withAlpha(p.color, 0) }]
  return { ...p, type, stops, transform: type === 'linear' ? linear(90) : RADIAL }
}

const replace = <T,>(list: T[], i: number, item: T) => list.map((v, j) => (j === i ? item : v))

function gradient(p: Fill) {
  const stops = p.stops.map((s) => `${hex(s.color)}${Math.round(alpha(s.color) * 2.55).toString(16).padStart(2, '0')} ${s.at * 100}%`)
  const shape = p.type === 'linear' ? `linear-gradient(${angle(p.transform) + 90}deg` : 'radial-gradient(circle'
  return `${shape}, ${stops.join(', ')})`
}

export function PaintList({
  title,
  paints,
  added,
  onChange,
  children,
}: {
  title: 'Fill' | 'Stroke'
  paints: Fill[]
  added: Fill
  onChange: (paints: Fill[]) => void
  children?: ReactNode
}) {
  const what = title.toLowerCase()
  return (
    <Section title={title} onAdd={() => onChange([...paints, added])}>
      {paints.length > 0 && (
        <ul className="rows">
          {paints
            .map((p, i) => ({ p, i }))
            .toReversed()
            .map(({ p, i }) => {
              const set = (next: Fill) => onChange(replace(paints, i, next))
              return (
                <li key={i} className="paint" data-hidden={!p.visible || undefined}>
                  <div className="row">
                    {p.type === 'solid' ? (
                      <Swatch label={`${title} color`} color={p.color} onChange={(color) => set({ ...p, color })} />
                    ) : (
                      <span className="swatch" aria-hidden="true" style={{ background: gradient(p) }} />
                    )}
                    <Select label={`${title} type`} value={p.type} options={TYPES} onChange={(t) => set(retype(p, t))} />
                    {p.type === 'solid' && (
                      <Field
                        label=""
                        title={`${title} opacity`}
                        unit="%"
                        value={alpha(p.color)}
                        onCommit={(v) => set({ ...p, color: withAlpha(p.color, v) })}
                      />
                    )}
                    <RowActions
                      what={what}
                      visible={p.visible}
                      onToggle={() => set({ ...p, visible: !p.visible })}
                      onRemove={() => onChange(paints.filter((_, j) => j !== i))}
                    />
                  </div>
                  {p.type !== 'solid' && (
                    <div className="stops">
                      {p.type === 'linear' && (
                        <Field
                          label="Angle"
                          unit="°"
                          value={angle(p.transform)}
                          onCommit={(v) => set({ ...p, transform: linear(v) })}
                        />
                      )}
                      {p.stops.map((s, k) => {
                        const stop = (next: Partial<typeof s>) => set({ ...p, stops: replace(p.stops, k, { ...s, ...next }) })
                        return (
                          <div key={k} className="row">
                            <Swatch label={`Stop ${k + 1} color`} color={s.color} onChange={(color) => stop({ color })} />
                            <Field
                              label=""
                              title={`Stop ${k + 1} position`}
                              unit="%"
                              value={s.at * 100}
                              onCommit={(v) => stop({ at: Math.min(1, Math.max(0, v / 100)) })}
                            />
                            <Field
                              label=""
                              title={`Stop ${k + 1} opacity`}
                              unit="%"
                              value={alpha(s.color)}
                              onCommit={(v) => stop({ color: withAlpha(s.color, v) })}
                            />
                          </div>
                        )
                      })}
                    </div>
                  )}
                </li>
              )
            })}
        </ul>
      )}
      {children}
    </Section>
  )
}

export function EffectList({ effects, onChange }: { effects: Effect[]; onChange: (effects: Effect[]) => void }) {
  return (
    <Section title="Effects" onAdd={() => onChange([...effects, SHADOW])}>
      {effects.length > 0 && (
        <ul className="rows">
          {effects.map((e, i) => {
            const set = (next: Partial<Effect>) => onChange(replace(effects, i, { ...e, ...next }))
            const what = EFFECTS[e.type].toLowerCase()
            return (
              <li key={i} className="paint" data-hidden={!e.visible || undefined}>
                <div className="row">
                  <Select
                    label="Effect type"
                    value={e.type}
                    options={EFFECTS}
                    onChange={(type) => set({ ...(type === 'blur' ? BLUR : SHADOW), visible: e.visible })}
                  />
                  <RowActions
                    what={what}
                    visible={e.visible}
                    onToggle={() => set({ visible: !e.visible })}
                    onRemove={() => onChange(effects.filter((_, j) => j !== i))}
                  />
                </div>
                <div className="grid">
                  {e.type === 'dropShadow' && (
                    <>
                      <Field label="X" title="Shadow X in mm" unit="mm" value={e.x / MM} onCommit={(v) => set({ x: v * MM })} />
                      <Field label="Y" title="Shadow Y in mm" unit="mm" value={e.y / MM} onCommit={(v) => set({ y: v * MM })} />
                    </>
                  )}
                  <Field
                    label="Blur"
                    unit="mm"
                    value={e.radius / MM}
                    onCommit={(v) => set({ radius: v * MM })}
                  />
                  {e.type === 'dropShadow' && (
                    <div className="row">
                      <Swatch label="Shadow color" color={e.color} onChange={(color) => set({ color })} />
                      <Field
                        label=""
                        title="Shadow opacity"
                        unit="%"
                        value={alpha(e.color)}
                        onCommit={(v) => set({ color: withAlpha(e.color, v) })}
                      />
                    </div>
                  )}
                </div>
              </li>
            )
          })}
        </ul>
      )}
    </Section>
  )
}
