import { useRef, useState, type CSSProperties, type ReactNode } from 'react'
import { ColorPicker } from './ColorPicker'
import { alpha, css, neutral, withAlpha, type ColorMode } from './color'
import { Field, RowActions, Section, Segmented, Select } from './controls'
import { Icon } from './icons'
import type { Editor } from './editor'
import type { Effect, Fill, Scope, Snapshot } from './model'

const TYPES = [['solid', 'Solid'], ['linear', 'Linear'], ['radial', 'Radial']] as const
const EFFECTS = { dropShadow: 'Drop shadow', blur: 'Layer blur' } as const
const RADIAL = [0.5, 0, 0, 0.5, 0.5, 0.5]
const shadow = (mode: ColorMode): Effect => ({
  type: 'dropShadow', x: 0, y: 4, radius: 4, color: withAlpha(neutral('black', mode), 25), visible: true,
})
const blur = (mode: ColorMode): Effect => ({ ...shadow(mode), type: 'blur', y: 0 })

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
const move = <T,>(list: T[], from: number, to: number) => {
  const out = list.filter((_, j) => j !== from)
  out.splice(to, 0, list[from])
  return out
}

export function gradient(p: Fill, scope: Scope) {
  const stops = p.stops.map((s) => `${css(s.color, scope)} ${s.at * 100}%`)
  const shape = p.type === 'linear' ? `linear-gradient(${angle(p.transform) + 90}deg` : 'radial-gradient(circle'
  return `${shape}, ${stops.join(', ')})`
}

export function PaintList({
  editor,
  title,
  paints,
  added,
  mode,
  scope,
  ppi,
  images = {},
  onChange,
  actions,
  children,
}: {
  editor: Editor
  title: 'Fill' | 'Text color' | 'Stroke'
  /** Null when the selected layers differ. */
  paints: Fill[] | null
  mode: ColorMode
  scope: Scope
  added: Fill
  /** Effective pixels per inch of the coarsest image among `paints`. */
  ppi?: number
  images?: Snapshot['images']
  onChange: (paints: Fill[]) => void
  actions?: ReactNode
  children?: ReactNode
}) {
  const [dragged, setDragged] = useState<number | null>(null)
  return (
    <Section title={title} actions={actions} onAdd={() => onChange([...(paints ?? []), added])}>
      {!paints && <p className="empty" title="Adding one replaces them on all layers">Mixed</p>}
      {paints && paints.length > 0 && (
        <ul className="rows">
          {paints
            .map((p, i) => ({ p, i }))
            .toReversed()
            .map(({ p, i }) => {
              const set = (next: Fill) => onChange(replace(paints, i, next))
              return (
                <li
                  key={i}
                  className="paint"
                  data-hidden={!p.visible || undefined}
                  data-dragged={dragged === i || undefined}
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.effectAllowed = 'move'
                    setDragged(i)
                  }}
                  onDragOver={(e) => dragged !== null && e.preventDefault()}
                  onDrop={() => dragged !== null && dragged !== i && onChange(move(paints, dragged, i))}
                  onDragEnd={() => setDragged(null)}
                >
                  <PaintRow
                    editor={editor}
                    title={title}
                    paint={p}
                    mode={mode}
                    scope={scope}
                    ppi={ppi}
                    images={images}
                    set={set}
                    onRemove={() => onChange(paints.filter((_, j) => j !== i))}
                  />
                </li>
              )
            })}
        </ul>
      )}
      {children}
    </Section>
  )
}

function PaintRow({
  editor,
  title,
  paint: p,
  mode,
  scope,
  ppi,
  images,
  set,
  onRemove,
}: {
  editor: Editor
  title: string
  paint: Fill
  mode: ColorMode
  scope: Scope
  ppi?: number
  images: Snapshot['images']
  set: (p: Fill) => void
  onRemove: () => void
}) {
  const [at, setAt] = useState(0)
  const k = Math.min(at, p.stops.length - 1)
  const solid = p.type === 'solid'
  const c = p.color
  const bound = solid && typeof c === 'object' && 'swatch' in c ? c : null
  const swatch = bound && scope.swatches.find((s) => s.id === bound.swatch)
  const boundVar = solid && typeof c === 'object' && 'variable' in c ? c : null
  const variable = boundVar && scope.variables.find((v) => v.id === boundVar.variable)
  return (
    <div className="row">
      {p.type === 'image' ? (
        <Icon name="image" />
      ) : (
        <ColorPicker
          label={`${title} color`}
          color={solid ? p.color : p.stops[k].color}
          mode={mode}
          scope={scope}
          editor={solid ? editor : undefined}
          chip={solid ? undefined : <span className="chip" style={{ background: gradient(p, scope) }} />}
          onChange={(color) => set(solid ? { ...p, color } : { ...p, stops: replace(p.stops, k, { ...p.stops[k], color }) })}
        >
          <Segmented label={`${title} type`} value={p.type} options={TYPES} onChange={(t) => set(retype(p, t))} />
          {!solid && <Stops paint={p} at={k} scope={scope} select={setAt} set={set} />}
        </ColorPicker>
      )}
      {p.type === 'image' ? (
        <span className="bound-name">
          Image{ppi !== undefined && ` · ${Math.round(ppi)} ppi`} · {images[p.image!]}
          {images[p.image!] === (mode === 'cmyk' ? 'RGB' : 'CMYK') && ` → ${mode.toUpperCase()}`}
        </span>
      ) : bound ? (
        <>
          <span className="bound-name" title={swatch ? swatch.name : 'Missing swatch'}>
            {swatch ? swatch.name : 'Missing swatch'}
          </span>
          <Field
            label={swatch?.spot ? 'Tint' : 'Opacity'}
            title={`${title} ${swatch?.spot ? 'tint' : 'opacity'}`}
            unit="%"
            max={100}
            value={swatch?.spot ? bound.tint * 100 : bound.alpha * 100}
            onCommit={(v) => {
              const x = v / 100
              set({ ...p, color: swatch?.spot ? { ...bound, tint: x } : { ...bound, alpha: x } })
            }}
          />
        </>
      ) : boundVar ? (
        <span className="bound-name" title={variable ? variable.name : 'Missing variable'}>
          <Icon name="drop" size={12} />
          {variable ? variable.name : 'Missing variable'}
        </span>
      ) : solid ? (
        <Field label="Opacity" title={`${title} opacity`} unit="%" value={alpha(p.color)} onCommit={(v) => set({ ...p, color: withAlpha(p.color, v) })} />
      ) : p.type === 'linear' ? (
        <Field label="Angle" title={`${title} angle`} unit="°" min={-Infinity} reset={0} value={angle(p.transform)} onCommit={(v) => set({ ...p, transform: linear(v) })} />
      ) : (
        <span className="bound-name">Radial</span>
      )}
      <RowActions what={title.toLowerCase()} visible={p.visible} onToggle={() => set({ ...p, visible: !p.visible })} onRemove={onRemove} />
    </div>
  )
}

/** A gradient bar with a marker per stop: a click on the bar adds a stop, a marker drags or takes arrow keys, Delete removes it. */
function Stops({ paint: p, at, scope, select, set }: { paint: Fill; at: number; scope: Scope; select: (k: number) => void; set: (p: Fill) => void }) {
  const bar = useRef<HTMLDivElement>(null)
  const where = (x: number) => {
    const r = bar.current!.getBoundingClientRect()
    return Math.min(1, Math.max(0, (x - r.left) / r.width))
  }
  const put = (k: number, v: number) => set({ ...p, stops: replace(p.stops, k, { ...p.stops[k], at: Math.min(1, Math.max(0, v)) }) })
  const remove = (k: number) => {
    if (p.stops.length < 3) return
    set({ ...p, stops: p.stops.filter((_, j) => j !== k) })
    select(0)
  }
  return (
    <div className="stops">
      <div
        ref={bar}
        className="picker-stop-bar"
        style={{ '--gradient': gradient({ ...p, type: 'linear', transform: linear(0) }, scope) } as CSSProperties}
        onPointerDown={(e) => {
          if (e.target !== e.currentTarget) return
          const t = where(e.clientX)
          const before = p.stops.filter((s) => s.at <= t).sort((a, b) => b.at - a.at)[0] ?? p.stops[0]
          set({ ...p, stops: [...p.stops, { ...before, at: t }] })
          select(p.stops.length)
        }}
      >
        {p.stops.map((s, k) => (
          <button
            key={k}
            type="button"
            className="picker-stop"
            aria-label={`Stop ${k + 1}`}
            title={`Stop ${k + 1}`}
            aria-pressed={k === at}
            onFocus={() => select(k)}
            style={{ left: `${s.at * 100}%`, '--color': css(s.color, scope) } as CSSProperties}
            onPointerDown={(e) => {
              select(k)
              e.currentTarget.setPointerCapture(e.pointerId)
            }}
            onPointerMove={(e) => e.currentTarget.hasPointerCapture(e.pointerId) && put(k, where(e.clientX))}
            onKeyDown={(e) => {
              const d = { ArrowLeft: -1, ArrowRight: 1 }[e.key]
              if (d) put(k, s.at + (d * (e.shiftKey ? 10 : 1)) / 100)
              else if (e.key === 'Delete' || e.key === 'Backspace') remove(k)
              else return
              e.preventDefault()
            }}
          />
        ))}
      </div>
      <div className="row">
        <Field label="Position" title="Stop position" unit="%" max={100} value={p.stops[at].at * 100} onCommit={(v) => put(at, v / 100)} />
        {p.stops.length > 2 && (
          <button type="button" className="icon-button" aria-label="Remove stop" title="Remove stop" onClick={() => remove(at)}>
            <Icon name="minus" />
          </button>
        )}
      </div>
    </div>
  )
}

export function EffectList({
  effects,
  mode,
  scope,
  onChange,
}: {
  /** Null when the selected layers differ. */
  effects: Effect[] | null
  mode: ColorMode
  scope: Scope
  onChange: (effects: Effect[]) => void
}) {
  return (
    <Section title="Effects" onAdd={() => onChange([...(effects ?? []), shadow(mode)])}>
      {!effects && <p className="empty" title="Adding one replaces them on all layers">Mixed</p>}
      {effects && effects.length > 0 && (
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
                    onChange={(type) => set({ ...(type === 'blur' ? blur : shadow)(mode), visible: e.visible })}
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
                      <Field label="Offset X" title="Shadow offset X" unit="length" min={-Infinity} value={e.x} onCommit={(x) => set({ x })} />
                      <Field label="Offset Y" title="Shadow offset Y" unit="length" min={-Infinity} value={e.y} onCommit={(y) => set({ y })} />
                    </>
                  )}
                  <Field
                    label="Blur"
                    unit="length"
                    value={e.radius}
                    onCommit={(radius) => set({ radius })}
                  />
                  {e.type === 'dropShadow' && (
                    <div className="row">
                      <ColorPicker label="Shadow color" color={e.color} mode={mode} scope={scope} onChange={(color) => set({ color })} />
                      <Field
                        label="Opacity"
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
