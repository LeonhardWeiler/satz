import {
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
  css,
  fromHsv,
  fromRgb,
  resolve,
  rgb as screen,
  toHsv,
  withAlpha,
  type Color,
  type ColorMode,
  type Hsv,
} from './color'
import { Field } from './controls'
import { Icon } from './icons'
import type { Scope, Swatch } from './model'
import { Popover, type Anchor } from './Popover'

const hex = (rgb: number) => rgb.toString(16).padStart(6, '0').toUpperCase()
const clamp = (v: number) => Math.min(1, Math.max(0, v))
const INKS = ['Cyan', 'Magenta', 'Yellow', 'Black']

type Props = { label: string; color: Color; mode: ColorMode; scope: Scope; onChange: (c: Color) => void }

/** Colour chip with a transparency checkerboard. */
export function Chip({ color, scope }: { color: Color; scope: Scope }) {
  return <span className="chip" style={{ '--color': css(color, scope) } as CSSProperties} />
}

export const NO_SCOPE: Scope = { swatches: [], collections: [], variables: [], modes: {} }

export function SwatchOption({ swatch, selected, onPick }: { swatch: Swatch; selected: boolean; onPick: () => void }) {
  const kind = swatch.spot ? 'Spot color' : `Process color (${typeof swatch.color === 'number' ? 'RGB' : 'CMYK'})`
  return (
    <button type="button" role="option" aria-selected={selected} className="swatch-option" title={`${swatch.name} · ${kind}`} onClick={onPick}>
      <Chip color={swatch.color} scope={NO_SCOPE} />
      <span className="swatch-name">{swatch.name}</span>
      <Icon name={swatch.spot ? 'spot' : 'process'} size={16} />
    </button>
  )
}

/** A swatch button that opens the picker; `bindable` adds the swatches tab. */
export function ColorPicker({ bindable, ...props }: Props & { bindable?: boolean }) {
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
        <Chip color={props.color} scope={props.scope} />
      </button>
      {open &&
        createPortal(
          <Picker
            {...props}
            anchor={() => {
              const b = button.current!.getBoundingClientRect()
              const panel = button.current!.closest('.panel')?.getBoundingClientRect() ?? b
              return new DOMRect(panel.x, b.y, panel.width, b.height)
            }}
            side="left"
            owner={button}
            tabs={bindable}
            onClose={(refocus) => {
              setOpen(null)
              if (refocus) button.current?.focus()
            }}
          />,
          open,
        )}
    </>
  )
}

export function Picker({
  anchor,
  side,
  owner,
  label,
  color,
  mode,
  scope,
  tabs,
  title = 'Custom',
  children,
  footer,
  onChange,
  onClose,
}: Props & {
  anchor: Anchor
  side: 'left' | 'right'
  owner?: RefObject<HTMLElement | null>
  tabs?: boolean
  title?: string
  children?: ReactNode
  footer?: ReactNode
  onClose: (refocus: boolean) => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  const bound = typeof color === 'object' ? ('swatch' in color ? color.swatch : 'variable' in color ? color.variable : null) : null
  const [tab, setTab] = useState<'custom' | 'swatches'>(bound ? 'swatches' : 'custom')
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
      if (!ref.current?.contains(t) && !owner?.current?.contains(t)) onClose(false)
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
    >
      <header className="picker-header">
        {tabs ? (
          <div role="tablist" className="tabs">
            {(['custom', 'swatches'] as const).map((t) => (
              <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
                {t === 'custom' ? 'Custom' : 'Swatches'}
              </button>
            ))}
          </div>
        ) : (
          <h2>{title}</h2>
        )}
        <button type="button" className="icon-button" aria-label="Close" title="Close" onClick={() => onClose(true)}>
          <Icon name="close" size={16} />
        </button>
      </header>
      {children}
      {tab === 'swatches' ? (
        <div role="listbox" aria-label="Swatches" className="swatch-list">
          {scope.swatches.length + colors.length === 0 && (
            <p className="empty">No swatches or colour variables yet. Add them in the Swatches panel or under Local variables.</p>
          )}
          {scope.collections.map((c) => {
            const vars = colors.filter((v) => v.collection === c.id)
            return (
              vars.length > 0 && (
                <div key={c.id} role="group" aria-label={c.name} className="swatch-group">
                  <h3>{c.name}</h3>
                  {vars.map((v) => (
                    <button
                      key={v.id}
                      type="button"
                      role="option"
                      aria-selected={v.id === bound}
                      className="swatch-option"
                      title={`${c.name} / ${v.name}`}
                      onClick={() => onChange({ variable: v.id, alpha: 1 })}
                    >
                      <Chip color={{ variable: v.id, alpha: 1 }} scope={scope} />
                      <span className="swatch-name">{v.name}</span>
                    </button>
                  ))}
                </div>
              )
            )
          })}
          {colors.length > 0 && scope.swatches.length > 0 && <h3>Swatches</h3>}
          {scope.swatches.map((sw) => (
            <SwatchOption
              key={sw.id}
              swatch={sw}
              selected={sw.id === bound}
              onPick={() => onChange({ swatch: sw.id, tint: 1, alpha: 1 })}
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
          {mode === 'cmyk' && typeof cmyk !== 'number' && 'cmyk' in cmyk ? (
            <div className="picker-inks">
              {INKS.map((ink, i) => (
                <Field
                  key={ink}
                  label={'CMYK'[i]}
                  title={ink}
                  unit=""
                  value={cmyk.cmyk[i] * 100}
                  onCommit={(p) => onChange({ ...cmyk, cmyk: cmyk.cmyk.map((c, j) => (j === i ? clamp(p / 100) : c)) })}
                />
              ))}
            </div>
          ) : (
            <div className="row">
              <label className="field">
                <span className="field-label">Hex</span>
                <input
                  key={rgb}
                  name="hex"
                  aria-label="Hex"
                  autoComplete="off"
                  spellCheck={false}
                  defaultValue={hex(rgb)}
                  onFocus={(e) => e.currentTarget.select()}
                  onBlur={(e) => commitHex(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') commitHex(e.currentTarget.value)
                  }}
                />
              </label>
              <Field
                label=""
                title="Opacity in %"
                unit="%"
                value={alpha(process)}
                onCommit={(p) => onChange(withAlpha(process, p))}
              />
            </div>
          )}
        </>
      )}
      {footer}
    </Popover>
  )
}
