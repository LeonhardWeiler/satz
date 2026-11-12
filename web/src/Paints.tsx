import { Fragment, useRef, useState, type CSSProperties, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ColorPicker } from './ColorPicker'
import { colorText, css, neutral, withAlpha, type ColorMode } from './color'
import { Field, RowActions, Section, Segmented, Select } from './controls'
import { Icon } from './icons'
import { useEditor, type Editor } from './editor'
import { ContextMenu } from './ContextMenu'
import { Dialog } from './Popover'
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
  image,
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
  /** The layer whose image paints get crop, fit and adjust. */
  image?: string
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
                    image={image}
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
  image,
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
  image?: string
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
        <span className="bound-name" title={imageText(ppi, images[p.image!], mode)}>
          {imageText(ppi, images[p.image!], mode)}
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
        <span className="bound-name" title={`${title} value`}>{colorText(p.color, scope)}</span>
      ) : p.type === 'linear' ? (
        <Field label="Angle" title={`${title} angle`} unit="°" min={-Infinity} reset={0} value={angle(p.transform)} onCommit={(v) => set({ ...p, transform: linear(v) })} />
      ) : (
        <span className="bound-name">Radial</span>
      )}
      {p.type === 'image' && image && <ImageTools editor={editor} id={image} paint={p} set={set} />}
      <RowActions what={title.toLowerCase()} visible={p.visible} onToggle={() => set({ ...p, visible: !p.visible })} onRemove={onRemove} />
    </div>
  )
}

const imageText = (ppi: number | undefined, space: string, mode: ColorMode) =>
  ['Image', ppi !== undefined && `${Math.round(ppi)} ppi`, space === (mode === 'cmyk' ? 'RGB' : 'CMYK') ? `${space} → ${mode.toUpperCase()}` : space].filter(Boolean).join(' · ')

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
          {effects.map((e, i) => (
            <EffectRow
              key={i}
              effect={e}
              mode={mode}
              scope={scope}
              set={(next) => onChange(replace(effects, i, { ...e, ...next }))}
              onRemove={() => onChange(effects.filter((_, j) => j !== i))}
            />
          ))}
        </ul>
      )}
    </Section>
  )
}

/** An effect's colour, type and visibility, and a button that opens its offset and blur. */
function EffectRow({ effect: e, mode, scope, set, onRemove }: { effect: Effect; mode: ColorMode; scope: Scope; set: (e: Partial<Effect>) => void; onRemove: () => void }) {
  const [at, setAt] = useState<Element | null>(null)
  const shadowed = e.type === 'dropShadow'
  return (
    <li className="paint" data-hidden={!e.visible || undefined}>
      <div className="row">
        {shadowed && <ColorPicker label="Shadow color" color={e.color} mode={mode} scope={scope} onChange={(color) => set({ color })} />}
        <Select label="Effect type" value={e.type} options={EFFECTS} onChange={(type) => set({ ...(type === 'blur' ? blur : shadow)(mode), visible: e.visible })} />
        <button type="button" className="icon-button" aria-label="Effect settings" title="Effect settings" aria-haspopup="dialog" aria-expanded={!!at} onClick={(ev) => setAt(ev.currentTarget)}>
          <Icon name="adjust" />
        </button>
        <RowActions what={EFFECTS[e.type].toLowerCase()} visible={e.visible} onToggle={() => set({ visible: !e.visible })} onRemove={onRemove} />
      </div>
      {at && (
        <Dialog label="Effect settings" at={at} onClose={() => setAt(null)}>
          <div className="grid">
            {shadowed && (
              <>
                <Field label="X" title="Shadow offset X" unit="length" min={-Infinity} value={e.x} onCommit={(x) => set({ x })} />
                <Field label="Y" title="Shadow offset Y" unit="length" min={-Infinity} value={e.y} onCommit={(y) => set({ y })} />
              </>
            )}
            <Field label="Blur" title="Blur" unit="length" value={e.radius} onCommit={(radius) => set({ radius })} />
          </div>
        </Dialog>
      )}
    </li>
  )
}

const FITS = [
  ['cover', 'Fill frame'],
  ['contain', 'Fit in frame'],
  ['frame', 'Frame to image'],
] as const
const ADJUST = ['Brightness', 'Contrast', 'Saturation']

const near = (a: number, b: number) => Math.abs(a - b) < 1e-4

/** How `fitImage` placed the image `p`, `frame` first when the frame has the image's aspect ratio. */
function fitOf(p: Fill) {
  const t = p.transform
  if (!near(t[1], 0) || !near(t[2], 0) || !near(t[4], (1 - t[0]) / 2) || !near(t[5], (1 - t[3]) / 2)) return undefined
  const [lo, hi] = [Math.min(t[0], t[3]), Math.max(t[0], t[3])]
  return near(lo, 1) && near(hi, 1) ? 'frame' : near(lo, 1) ? 'cover' : near(hi, 1) ? 'contain' : undefined
}

/** Crop, fit and the brightness, contrast and saturation of the image `p` of the layer `id`. */
function ImageTools({ editor, id, paint: p, set }: { editor: Editor; id: string; paint: Fill; set: (p: Fill) => void }) {
  const cropping = useEditor(editor, (e) => e.cropping)
  const [fitAt, setFitAt] = useState<DOMRect | null>(null)
  const [adjustAt, setAdjustAt] = useState<Element | null>(null)
  const adjust = p.adjust ?? [0, 0, 0]
  const change = (i: number, v: number) => set({ ...p, adjust: adjust.with(i, v / 100) as Fill['adjust'] })
  return (
    <>
      <button type="button" className="icon-button" title="Crop" aria-pressed={cropping === id} onClick={() => editor.set({ cropping: cropping === id ? null : id })}>
        <Icon name="crop" />
      </button>
      <button type="button" className="icon-button" title="Fit image" aria-haspopup="menu" onClick={(e) => setFitAt(e.currentTarget.getBoundingClientRect())}>
        <Icon name="fit" />
      </button>
      <button type="button" className="icon-button" title="Adjust image" aria-haspopup="dialog" aria-expanded={!!adjustAt} onClick={(e) => setAdjustAt(e.currentTarget)}>
        <Icon name="adjust" />
      </button>
      {fitAt &&
        createPortal(
          <ContextMenu
            anchor={() => fitAt}
            side="bottom"
            label="Fit image"
            onClose={() => setFitAt(null)}
            items={FITS.map(([fit, name]) => [name, () => editor.apply({ type: 'fitImage', id, fit }), true, fit === fitOf(p)])}
          />,
          document.body,
        )}
      {adjustAt && (
        <Dialog label="Adjust image" at={adjustAt} onClose={() => setAdjustAt(null)}>
          <div className="adjust">
            {ADJUST.map((name, i) => (
              <Fragment key={name}>
                <span>{name}</span>
                <input type="range" aria-label={name} min={-100} max={100} value={Math.round(adjust[i] * 100)} onChange={(e) => change(i, Number(e.currentTarget.value))} />
                <Field label="" title={name} unit="%" min={-100} max={100} reset={0} value={Math.round(adjust[i] * 100)} onCommit={(v) => change(i, v)} />
              </Fragment>
            ))}
            <button type="button" className="button" disabled={adjust.every((v) => !v)} onClick={() => set({ ...p, adjust: [0, 0, 0] })}>
              Reset
            </button>
          </div>
        </Dialog>
      )}
    </>
  )
}
