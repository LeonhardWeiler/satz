import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent, type RefObject } from 'react'
import { createPortal } from 'react-dom'
import { alpha, css, fromHsv, fromRgb, rgb as screen, toHsv, withAlpha, type Color, type ColorMode, type Hsv } from './color'
import { Field } from './controls'
import { Icon } from './icons'
import { Popover, type Anchor } from './Popover'

const hex = (rgb: number) => rgb.toString(16).padStart(6, '0').toUpperCase()
const clamp = (v: number) => Math.min(1, Math.max(0, v))
const INKS = ['Cyan', 'Magenta', 'Yellow', 'Black']

type Props = { label: string; color: Color; mode: ColorMode; onChange: (c: Color) => void }

export function ColorPicker({ label, color, mode, onChange }: Props) {
  const [open, setOpen] = useState(false)
  const button = useRef<HTMLButtonElement>(null)
  return (
    <>
      <button
        ref={button}
        type="button"
        className="swatch"
        aria-label={label}
        title={label}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span style={{ '--color': css(color) } as CSSProperties} />
      </button>
      {open &&
        createPortal(
          <Picker
            anchor={() => {
              const b = button.current!.getBoundingClientRect()
              const panel = button.current!.closest('.panel')!.getBoundingClientRect()
              return new DOMRect(panel.x, b.y, panel.width, b.height)
            }}
            button={button}
            label={label}
            color={color}
            mode={mode}
            onChange={onChange}
            onClose={(refocus) => {
              setOpen(false)
              if (refocus) button.current?.focus()
            }}
          />,
          document.body,
        )}
    </>
  )
}

function Picker({
  anchor,
  button,
  label,
  color,
  mode,
  onChange,
  onClose,
}: Props & { anchor: Anchor; button: RefObject<HTMLElement | null>; onClose: (refocus: boolean) => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const rgb = screen(color)
  const [state, setState] = useState(() => ({ color, hsv: toHsv(rgb) }))
  if (state.color !== color) setState({ color, hsv: rgb === screen(state.color) ? state.hsv : toHsv(rgb) })
  const [h, s, v] = state.hsv

  const set = (hsv: Hsv) => {
    const next = fromRgb(fromHsv(hsv), color, mode)
    setState({ color: next, hsv })
    onChange(next)
  }
  const cmyk = typeof color === 'number' ? fromRgb(rgb, color, 'cmyk') : color

  useEffect(() => ref.current?.focus(), [])

  useEffect(() => {
    const down = (e: globalThis.PointerEvent) => {
      const t = e.target as Node
      if (!ref.current?.contains(t) && !button.current?.contains(t)) onClose(false)
    }
    document.addEventListener('pointerdown', down, true)
    return () => document.removeEventListener('pointerdown', down, true)
  }, [button, onClose])

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
      side="left"
      className="picker"
      role="dialog"
      aria-label={label}
      tabIndex={-1}
      onKeyDown={(e) => {
        if (e.key !== 'Escape') return
        e.stopPropagation()
        onClose(true)
      }}
    >
      <header className="picker-header">
        <h2>Custom</h2>
        <button type="button" className="icon-button" aria-label="Close" title="Close" onClick={() => onClose(true)}>
          <Icon name="close" size={16} />
        </button>
      </header>
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
          value={Math.round(alpha(color))}
          style={{ '--color': `#${hex(rgb)}` } as CSSProperties}
          onChange={(e) => onChange(withAlpha(color, Number(e.currentTarget.value)))}
        />
      </div>
      {mode === 'cmyk' && typeof cmyk !== 'number' ? (
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
        <Field label="" title="Opacity in %" unit="%" value={alpha(color)} onCommit={(p) => onChange(withAlpha(color, p))} />
      </div>
      )}
    </Popover>
  )
}
