import {
  Fragment,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
  type ReactNode,
  type RefObject,
} from 'react'
import { createPortal } from 'react-dom'
import {
  alpha,
  cmykText,
  css,
  fromHsv,
  fromRgb,
  parseCmyk,
  resolve,
  rgb as screen,
  toHsv,
  withAlpha,
  type Color,
  type ColorMode,
  type Hsv,
} from './color'
import { Field, nextName, Select } from './controls'
import type { Editor } from './editor'
import { Icon } from './icons'
import type { Scope, Swatch } from './model'
import { docked, Popover, type Anchor } from './Popover'

const hex = (rgb: number) => rgb.toString(16).padStart(6, '0').toUpperCase()
const clamp = (v: number) => Math.min(1, Math.max(0, v))
const INKS = ['Cyan', 'Magenta', 'Yellow', 'Black']
const INK_COLORS = ['#00a0e3', '#e5007e', '#ffed00', '#1d1d1b']

type Props = { label: string; color: Color; mode: ColorMode; scope: Scope; onChange: (c: Color) => void }

/** Color chip with a transparency checkerboard. */
export function Chip({ color, scope }: { color: Color; scope: Scope }) {
  return <span className="chip" style={{ '--color': css(color, scope) } as CSSProperties} />
}

export const NO_SCOPE: Scope = { swatches: [], collections: [], variables: [], modes: {} }

/** A swatch color's CMYK values in %, or RGB. */
export const ink = (c: Color) => (typeof c === 'object' && 'cmyk' in c ? c.cmyk.map((v, i) => 'CMYK'[i] + Math.round(v * 100)).join(' ') : 'RGB')

export function SwatchOption({ swatch, selected, onPick }: { swatch: Swatch; selected: boolean; onPick: () => void }) {
  const values = ink(swatch.color)
  const kind = swatch.spot ? 'Spot color' : 'Process color'
  const inks = values === 'RGB' ? 'RGB only, converted to CMYK when printed' : values
  return (
    <button type="button" role="option" aria-selected={selected} className="swatch-option" title={`${swatch.name} · ${kind} · ${inks}`} onClick={onPick}>
      <Chip color={swatch.color} scope={NO_SCOPE} />
      <span className="swatch-name">{swatch.name}</span>
      <span className="swatch-ink" data-ink={values} />
      {values === 'RGB' ? (
        <span className="swatch-warn">
          <Icon name="warn" />
        </span>
      ) : (
        swatch.spot ? <span className="badge">Spot</span> : <span />
      )}
    </button>
  )
}

/** A swatch button that opens the picker; `editor` adds the swatches tab, `chip` replaces the colour chip, `children` sit above the colour. */
export function ColorPicker({ chip, children, ...props }: Props & { editor?: Editor; chip?: ReactNode; children?: ReactNode }) {
  const [open, setOpen] = useState<Element | null>(null)
  const button = useRef<HTMLButtonElement>(null)
  return (
    <>
      <button
        ref={button}
        type="button"
        className="swatch"
        aria-label={props.label}
        title={props.label}
        aria-haspopup="dialog"
        aria-expanded={!!open}
        onClick={(e) => {
          const into = e.currentTarget.closest('dialog') ?? document.body
          setOpen((o) => (o ? null : into))
        }}
      >
        {chip ?? <Chip color={props.color} scope={props.scope} />}
      </button>
      {open &&
        createPortal(
          <Picker
            {...props}
            anchor={() => docked(button.current!)()}
            side="left"
            owner={button}
            onClose={(refocus) => {
              setOpen(null)
              if (refocus) button.current?.focus()
            }}
          >
            {children}
          </Picker>,
          open,
        )}
    </>
  )
}

const HARMONIES = { complementary: [0, 180], analogous: [-30, 0, 30], triadic: [0, 120, 240], split: [0, 150, 210], square: [0, 90, 180, 270] }
type Harmony = keyof typeof HARMONIES
const TABS = { custom: 'Custom', harmony: 'Harmony', swatches: 'Swatches' }
const HARMONY_NAMES: Record<Harmony, string> = { complementary: 'Complementary', analogous: 'Analogous', triadic: 'Triadic', split: 'Split complementary', square: 'Square' }

/** The colours of `rule` around the hue of `hsv`, at its saturation and brightness. */
const harmony = ([h, s, v]: Hsv, rule: Harmony) => HARMONIES[rule].map((o): Hsv => [(h + o + 360) % 360, s, v])

/** Hue around and saturation out from the centre, with the colours of `rule` marked; only the colour itself moves. */
function Wheel({ hsv, rule, set }: { hsv: Hsv; rule: Harmony; set: (hsv: Hsv) => void }) {
  const [h, s, v] = hsv
  const pick = (e: PointerEvent<HTMLDivElement>) => {
    if (!(e.buttons & 1)) return
    const r = e.currentTarget.getBoundingClientRect()
    const [x, y] = [((e.clientX - r.left) / r.width) * 2 - 1, ((e.clientY - r.top) / r.height) * 2 - 1]
    set([((Math.atan2(x, -y) * 180) / Math.PI + 360) % 360, clamp(Math.hypot(x, y)), v])
  }
  const step = (e: KeyboardEvent<HTMLDivElement>) => {
    const d = e.shiftKey ? 10 : 1
    const move = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowDown: [0, -d / 100], ArrowUp: [0, d / 100] }[e.key]
    if (!move) return
    e.preventDefault()
    set([(h + move[0] + 360) % 360, clamp(s + move[1]), v])
  }
  return (
    <div
      className="picker-wheel"
      role="slider"
      tabIndex={0}
      aria-label="Hue and saturation"
      aria-valuenow={Math.round(h)}
      aria-valuetext={`Hue ${Math.round(h)}°, saturation ${Math.round(s * 100)}%`}
      style={{ '--dark': 1 - v } as CSSProperties}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId)
        pick(e)
      }}
      onPointerMove={pick}
      onKeyDown={step}
    >
      {harmony(hsv, rule).map((c, i) => {
        const a = (c[0] * Math.PI) / 180
        return (
          <span
            key={i}
            className="picker-thumb"
            data-own={HARMONIES[rule][i] === 0 || undefined}
            style={{ left: `${50 + 50 * s * Math.sin(a)}%`, top: `${50 - 50 * s * Math.cos(a)}%`, background: `#${hex(fromHsv(c))}` }}
          />
        )
      })}
    </div>
  )
}

export function Picker({
  anchor,
  side,
  owner,
  editor,
  label,
  color,
  mode,
  scope,
  title = label,
  children,
  onChange,
  onClose,
}: Props & {
  anchor: Anchor
  side: 'left' | 'right'
  owner?: RefObject<HTMLElement | null>
  editor?: Editor
  title?: string
  children?: ReactNode
  onClose: (refocus: boolean) => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  const bound = typeof color === 'object' ? ('swatch' in color ? color.swatch : 'variable' in color ? color.variable : null) : null
  const [tab, setTab] = useState<'custom' | 'harmony' | 'swatches'>(bound ? 'swatches' : 'custom')
  const [rule, setRule] = useState<Harmony>('complementary')
  const process = resolve(color, scope)
  const rgb = screen(process, scope)
  const [state, setState] = useState(() => ({ color, hsv: toHsv(rgb) }))
  if (state.color !== color) setState({ color, hsv: rgb === screen(state.color, scope) ? state.hsv : toHsv(rgb) })
  const colors = scope.variables.filter((v) => 'color' in Object.values(v.values)[0])
  const [h, s, v] = state.hsv

  const set = (hsv: Hsv) => {
    const next = fromRgb(fromHsv(hsv), process, mode)
    setState({ color: next, hsv })
    onChange(next)
  }
  const cmyk = typeof process === 'number' ? fromRgb(rgb, process, 'cmyk') : process

  useEffect(() => ref.current?.focus(), [])

  useEffect(() => {
    const down = (e: globalThis.PointerEvent) => {
      const t = e.target as Node
      if (!ref.current?.contains(t) && !owner?.current?.contains(t) && !(t as Element).closest?.('[role=menu]')) onClose(false)
    }
    const key = (e: globalThis.KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.preventDefault()
      e.stopPropagation()
      onClose(true)
    }
    document.addEventListener('pointerdown', down, true)
    document.addEventListener('keydown', key, true)
    return () => {
      document.removeEventListener('pointerdown', down, true)
      document.removeEventListener('keydown', key, true)
    }
  }, [owner, onClose])

  const pick = (e: PointerEvent<HTMLDivElement>) => {
    if (!(e.buttons & 1)) return
    const r = e.currentTarget.getBoundingClientRect()
    set([h, clamp((e.clientX - r.left) / r.width), clamp(1 - (e.clientY - r.top) / r.height)])
  }
  const step = (e: KeyboardEvent<HTMLDivElement>) => {
    const d = e.shiftKey ? 0.1 : 0.01
    const move = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowDown: [0, -d], ArrowUp: [0, d] }[e.key]
    if (!move) return
    e.preventDefault()
    set([h, clamp(s + move[0]), clamp(v + move[1])])
  }
  const commitHex = (text: string) => {
    const t = text.replace(/^#/, '')
    const full = t.length === 3 ? [...t].map((c) => c + c).join('') : t
    if (/^[0-9a-f]{6}$/i.test(full)) set(toHsv(parseInt(full, 16)))
  }

  return (
    <Popover
      ref={ref}
      anchor={anchor}
      side={side}
      className="picker"
      role="dialog"
      aria-label={label}
      tabIndex={-1}
      onCopy={(e) => {
        if (mode !== 'cmyk' || typeof cmyk === 'number' || !('cmyk' in cmyk)) return
        e.clipboardData.setData('text/plain', cmykText(cmyk.cmyk))
        e.preventDefault()
      }}
      onPaste={(e) => {
        const values = mode === 'cmyk' ? parseCmyk(e.clipboardData.getData('text/plain')) : null
        if (!values) return
        e.preventDefault()
        onChange({ cmyk: values, alpha: alpha(process) / 100 })
      }}
    >
      <header className="picker-header">
        <h2>{title}</h2>
        {editor && (
          <button
            type="button"
            className="icon-button"
            aria-label="Add to swatches"
            title="Add to swatches"
            onClick={() => {
              const name = nextName('Swatch', scope.swatches.map((s) => s.name))
              const [swatch] = editor.apply({ type: 'addSwatch', name, color: withAlpha(process, 100), spot: false })
              onChange({ swatch, tint: 1, alpha: alpha(process) / 100 })
              setTab('swatches')
            }}
          >
            <Icon name="plus" />
          </button>
        )}
      </header>
      {children}
      <div role="tablist" className="tabs">
        {(editor ? (['custom', 'harmony', 'swatches'] as const) : (['custom', 'harmony'] as const)).map((t) => (
          <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
            {TABS[t]}
          </button>
        ))}
      </div>
      {tab === 'harmony' ? (
        <>
          <Wheel hsv={state.hsv} rule={rule} set={set} />
          <Select label="Harmony" value={rule} options={HARMONY_NAMES} onChange={setRule} />
          <div className="picker-harmony" role="group" aria-label="Harmony colors">
            {harmony(state.hsv, rule).map((c, i) => (
              <button key={i} type="button" aria-label={`Harmony color #${hex(fromHsv(c))}`} title={`#${hex(fromHsv(c))}`} style={{ background: `#${hex(fromHsv(c))}` }} onClick={() => set(c)} />
            ))}
          </div>
          {editor && (
            <button
              type="button"
              className="button"
              onClick={() =>
                editor.batch(() => {
                  const names = scope.swatches.map((s) => s.name)
                  for (const c of harmony(state.hsv, rule)) {
                    names.push(nextName('Swatch', names))
                    editor.apply({ type: 'addSwatch', name: names.at(-1)!, color: withAlpha(fromRgb(fromHsv(c), process, mode), 100), spot: false })
                  }
                  setTab('swatches')
                })
              }
            >
              Add harmony to swatches
            </button>
          )}
        </>
      ) : tab === 'swatches' ? (
        <div role="listbox" aria-label="Swatches" className="swatch-list">
          {scope.swatches.length + colors.length === 0 && (
            <p className="empty">No swatches or color variables yet.</p>
          )}
          {colors.length > 0 && <h3>Variables</h3>}
          {colors.map((v) => (
            <button
              key={v.id}
              type="button"
              role="option"
              aria-selected={v.id === bound}
              className="swatch-option"
              title={v.name}
              onClick={() => {
                onChange({ variable: v.id, alpha: 1 })
                onClose(true)
              }}
            >
              <Chip color={{ variable: v.id, alpha: 1 }} scope={scope} />
              <span className="swatch-name">{v.name}</span>
            </button>
          ))}
          {colors.length > 0 && scope.swatches.length > 0 && <h3>Swatches</h3>}
          {scope.swatches.map((sw) => (
            <SwatchOption
              key={sw.id}
              swatch={sw}
              selected={sw.id === bound}
              onPick={() => {
                onChange({ swatch: sw.id, tint: 1, alpha: 1 })
                onClose(true)
              }}
            />
          ))}
        </div>
      ) : (
        <>
          <div
            className="picker-area"
            role="slider"
            tabIndex={0}
            aria-label="Saturation and brightness"
            aria-valuenow={Math.round(s * 100)}
            aria-valuetext={`Saturation ${Math.round(s * 100)}%, brightness ${Math.round(v * 100)}%`}
            style={{ backgroundColor: `hsl(${h} 100% 50%)` }}
            onPointerDown={(e) => {
              e.currentTarget.setPointerCapture(e.pointerId)
              pick(e)
            }}
            onPointerMove={pick}
            onKeyDown={step}
          >
            <span className="picker-thumb" style={{ left: `${s * 100}%`, top: `${(1 - v) * 100}%`, background: `#${hex(rgb)}` }} />
          </div>
          <div className="picker-sliders">
            <input
              type="range"
              className="picker-hue"
              aria-label="Hue"
              min={0}
              max={360}
              value={h}
              onChange={(e) => set([Number(e.currentTarget.value), s, v])}
            />
            <input
              type="range"
              className="picker-alpha"
              aria-label="Opacity"
              min={0}
              max={100}
              value={Math.round(alpha(process))}
              style={{ '--color': `#${hex(rgb)}` } as CSSProperties}
              onChange={(e) => onChange(withAlpha(process, Number(e.currentTarget.value)))}
            />
          </div>
          {mode === 'cmyk' && typeof cmyk !== 'number' && 'cmyk' in cmyk && (
            <div className="picker-inks">
              {INKS.map((ink, i) => {
                const put = (p: number) => onChange({ ...cmyk, cmyk: cmyk.cmyk.map((c, j) => (j === i ? clamp(p / 100) : c)) })
                return (
                  <Fragment key={ink}>
                    <input type="range" aria-label={ink} style={{ '--ink': INK_COLORS[i] } as CSSProperties} min={0} max={100} value={Math.round(cmyk.cmyk[i] * 100)} onChange={(e) => put(Number(e.currentTarget.value))} />
                    <Field label={'CMYK'[i]} title={ink} unit="" value={Math.round(cmyk.cmyk[i] * 100)} onCommit={put} />
                  </Fragment>
                )
              })}
            </div>
          )}
          <div className="row">
            <label className="field">
              <span className="field-label">Hex</span>
              <input
                key={rgb}
                name="hex"
                aria-label="Hex"
                autoComplete="off"
                spellCheck={false}
                defaultValue={`#${hex(rgb)}`}
                onFocus={(e) => e.currentTarget.select()}
                onBlur={(e) => commitHex(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') commitHex(e.currentTarget.value)
                }}
              />
            </label>
          </div>
        </>
      )}
    </Popover>
  )
}
