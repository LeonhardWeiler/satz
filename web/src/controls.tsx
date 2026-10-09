import { Fragment, useId, useRef, useState, type KeyboardEvent, type MouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { keyName } from './commands'
import { ContextMenu } from './ContextMenu'
import { evalExpr, step } from './field'
import { Icon, type IconName } from './icons'
import type { Typeface } from './model'
import { UNITS, useSettings } from './settings'

/** Moves the focus from the event's target to the item before or after it for the keys `back` and `forth`. */
export function roam(e: KeyboardEvent, items: Element[], back = ['ArrowUp'], forth = ['ArrowDown']) {
  const i = items.indexOf(e.target as Element)
  const d = back.includes(e.key) ? -1 : forth.includes(e.key) ? 1 : 0
  if (i < 0 || !d) return false
  e.preventDefault()
  ;(items[i + d] as HTMLElement | undefined)?.focus()
  return true
}

const round = (v: number, unit: string) => (unit === '%' ? Math.round(v) : Math.round(v * 100) / 100)

/**
 * A number input that takes expressions with units. ↑/↓ step it, Shift by 10 and Alt by 0.1;
 * dragging its label scrubs it and double-clicking it sets `reset`; invalid input shakes and keeps the
 * value. Values stay within `min` and `max`, in `unit`.
 */
export function Field({
  label,
  value,
  unit: given,
  onCommit,
  readOnly,
  zero,
  int,
  min = 0,
  max = Infinity,
  reset,
  disabled,
  title: name = typeof label === 'string' ? label : undefined,
}: {
  label: ReactNode
  /** In pt for lengths, shown in `unit`. */
  value: number | null
  /** "length" for the length unit of the settings. */
  unit: string
  onCommit?: (v: number) => void
  readOnly?: boolean
  /** Shown for 0 and accepted as input for it, e.g. "Auto". */
  zero?: string
  /** Whole numbers with − and + buttons. */
  int?: boolean
  min?: number
  max?: number
  /** The default value, in pt for lengths. */
  reset?: number
  disabled?: boolean
  title?: string
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const [bad, setBad] = useState(false)
  const [scrub, setScrub] = useState(false)
  const id = useId()
  const lengths = useSettings().unit
  const unit = given === 'length' ? lengths : given
  const per = given === 'length' ? UNITS[lengths] : 1
  const title = name && unit ? `${name} in ${unit}` : (name ?? unit)
  const shown = value === null ? null : round(value / per, unit)
  const put = (v: number) => {
    const c = zero && v === 0 ? 0 : Math.min(max, Math.max(min, v))
    onCommit?.((int ? Math.round(c) : round(c, unit)) * per)
  }
  const edit = !readOnly && !disabled && onCommit && shown !== null
  const cur = shown ?? 0
  const commit = () => {
    setDraft(null)
    if (draft === null) return
    const v = zero && ['', zero.toLowerCase()].includes(draft.trim().toLowerCase()) ? 0 : evalExpr(draft, cur, unit)
    if (v === null) setBad(true)
    else put(v)
  }
  const bump = (d: number) => edit && put(cur + d)
  return (
    <label
      htmlFor={id}
      className={`field${int ? ' int' : ''}${bad ? ' bad' : ''}${scrub ? ' scrub' : ''}${disabled ? ' disabled' : ''}`}
      title={title}
      onAnimationEnd={() => setBad(false)}
    >
      {label && (
        <span
          className={`field-label${edit ? ' scrubs' : ''}`}
          onClick={(e) => e.preventDefault()}
          onDoubleClick={() => edit && reset !== undefined && onCommit(reset)}
          onPointerDown={(e) => {
            if (!edit) return
            e.preventDefault()
            const x0 = e.clientX
            const target = e.currentTarget
            target.setPointerCapture(e.pointerId)
            setScrub(true)
            const move = (m: PointerEvent) => put(cur + Math.round((m.clientX - x0) / 2) * step(m, int))
            target.addEventListener('pointermove', move)
            target.addEventListener(
              'pointerup',
              () => {
                target.removeEventListener('pointermove', move)
                setScrub(false)
              },
              { once: true },
            )
          }}
        >
          {label}
        </span>
      )}
      {int && edit && (
        <button type="button" className="field-step" tabIndex={-1} aria-label={`Decrease ${title}`} onClick={(e) => bump(-step(e, true))}>
          <Icon name="minus" />
        </button>
      )}
      <input
        id={id}
        name={title.toLowerCase().replaceAll(' ', '-')}
        aria-label={title}
        inputMode="decimal"
        autoComplete="off"
        spellCheck={false}
        readOnly={readOnly}
        disabled={disabled}
        placeholder={zero}
        value={draft ?? (shown === null ? 'Mixed' : shown === 0 && zero ? '' : String(shown))}
        onChange={(e) => {
          setDraft(e.currentTarget.value)
          setBad(false)
        }}
        onFocus={(e) => e.currentTarget.select()}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            e.currentTarget.blur()
          } else if (e.key === 'Escape') {
            setDraft(null)
            e.currentTarget.blur()
          } else if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && edit) {
            e.preventDefault()
            setDraft(null)
            put(cur + (e.key === 'ArrowUp' ? 1 : -1) * step(e, int))
            const input = e.currentTarget
            requestAnimationFrame(() => input.select())
          }
        }}
      />
      {((shown !== null && !(shown === 0 && zero)) || draft !== null) && unit && <span className="field-unit">{unit}</span>}
      {int && edit && (
        <button type="button" className="field-step" tabIndex={-1} aria-label={`Increase ${title}`} onClick={(e) => bump(step(e, true))}>
          <Icon name="plus" />
        </button>
      )}
    </label>
  )
}

/** A text input that commits a changed, non-empty value on blur or Enter. */
export function NameInput({ label, value, autoFocus, blank, onCommit }: { label: string; value: string; autoFocus?: boolean; blank?: boolean; onCommit: (v: string) => void }) {
  return (
    <input
      key={value}
      autoFocus={autoFocus}
      onFocus={(e) => autoFocus && e.currentTarget.select()}
      className="name-input"
      name={label.toLowerCase().replaceAll(' ', '-')}
      aria-label={label}
      autoComplete="off"
      spellCheck={false}
      defaultValue={value}
      onBlur={(e) => {
        const v = e.currentTarget.value.trim()
        if ((v || blank) && v !== value) onCommit(v)
        else e.currentTarget.value = value
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === 'Escape') {
          e.preventDefault()
          if (e.key === 'Escape') e.currentTarget.value = value
          e.currentTarget.blur()
        }
      }}
    />
  )
}

/** The next free `prefix n` among `names`. */
export const nextName = (prefix: string, names: string[]) =>
  `${prefix} ${Math.max(0, ...names.map((n) => (n.startsWith(`${prefix} `) ? Number(n.slice(prefix.length + 1)) || 0 : 0))) + 1}`

/** A button that shows the option of `value`, or Mixed for null, and picks another from a menu. */
export function Select<T extends string>({
  label,
  value,
  options,
  disabled = [],
  breaks = [],
  prefix,
  search,
  onChange,
}: {
  label: string
  value: T | null
  options: Record<T, ReactNode>
  disabled?: T[]
  /** Options followed by a separator. */
  breaks?: T[]
  /** Shown before the value, like a field's label. */
  prefix?: ReactNode
  search?: boolean
  onChange: (v: T) => void
}) {
  const ref = useRef<HTMLButtonElement>(null)
  const [host, setHost] = useState<Element | null>(null)
  const open = (e: { currentTarget: Element }) => setHost(e.currentTarget.closest('dialog') ?? document.body)
  return (
    <>
      <button
        ref={ref}
        type="button"
        className="select"
        role="combobox"
        aria-label={label}
        aria-expanded={!!host}
        aria-haspopup="menu"
        disabled={Object.keys(options).length < 2 && value !== null}
        onClick={open}
        onKeyDown={(e) => {
          if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
          e.preventDefault()
          open(e)
        }}
      >
        {prefix && <span className="select-prefix">{prefix}</span>}
        <span>{value === null ? 'Mixed' : options[value]}</span>
        <Icon name="chevron" />
      </button>
      {host &&
        createPortal(
          <ContextMenu
            anchor={() => ref.current!.getBoundingClientRect()}
            side="bottom"
            label={label}
            search={search}
            onClose={() => setHost(null)}
            items={(Object.keys(options) as T[]).flatMap((v) => [[options[v], () => onChange(v), !disabled.includes(v), v === value] as const, ...(breaks.includes(v) ? [null] : [])])}
          />,
          host,
        )}
    </>
  )
}

/** Family and style of the font `value`, a hash of `faces`, or Mixed for null; the missing ones are listed but not picked. */
export function FontSelect({ label, faces, missing = [], value, compact, onChange }: { label: string; faces: Typeface[]; missing?: string[]; value: string | null; compact?: boolean; onChange: (f: Typeface) => void }) {
  const family = (f: Typeface) => f.family || f.name
  const face = faces.find((f) => f.hash === value)
  const families = Object.fromEntries(faces.map((f) => [family(f), family(f)]))
  const styles = faces.filter((f) => face && family(f) === family(face))
  const pick = (name: string) => {
    const all = faces.filter((f) => family(f) === name && !missing.includes(f.hash))
    const styled = (s?: string) => all.find((f) => f.style === s)
    const f = styled(face?.style) ?? styled('Regular') ?? all[0]
    if (f) onChange(f)
  }
  if (compact)
    return (
      <Select
        label={label}
        value={face?.hash ?? null}
        options={Object.fromEntries(faces.map((f) => [f.hash, f.family ? `${f.family} ${f.style}` : f.name]))}
        disabled={missing}
        search
        onChange={(hash) => onChange(faces.find((f) => f.hash === hash)!)}
      />
    )
  return (
    <div className="grid">
      <Select label={label} value={face ? family(face) : null} options={families} search onChange={pick} />
      <Select
        label={`${label} style`}
        value={face?.hash ?? null}
        options={Object.fromEntries(styles.map((f) => [f.hash, f.style || f.name]))}
        disabled={missing}
        onChange={(hash) => onChange(faces.find((f) => f.hash === hash)!)}
      />
    </div>
  )
}

export function Section({
  title,
  onAdd,
  actions,
  children,
}: {
  title: string
  onAdd?: (e: MouseEvent<HTMLButtonElement>) => void
  actions?: ReactNode
  children?: ReactNode
}) {
  const add = `Add ${title.toLowerCase().replace(/s$/, '')}`
  return (
    <section className="section" aria-label={title}>
      <header className="section-header">
        <h3>{title}</h3>
        {actions}
        {onAdd && (
          <button type="button" className="icon-button" aria-label={add} title={add} onClick={onAdd}>
            <Icon name="plus" />
          </button>
        )}
      </header>
      <div className="section-body">{children}</div>
    </section>
  )
}

/** Eye and minus buttons at the end of a fill, stroke or effect row. */
export function RowActions({
  what,
  visible,
  onToggle,
  onRemove,
}: {
  what: string
  visible: boolean
  onToggle: () => void
  onRemove: () => void
}) {
  return (
    <>
      <button
        type="button"
        className="icon-button"
        aria-label={`${visible ? 'Hide' : 'Show'} ${what}`}
        title={`${visible ? 'Hide' : 'Show'} ${what}`}
        onClick={onToggle}
      >
        <Icon name={visible ? 'eye' : 'eyeOff'} />
      </button>
      <button type="button" className="icon-button" aria-label={`Remove ${what}`} title={`Remove ${what}`} onClick={onRemove}>
        <Icon name="minus" />
      </button>
    </>
  )
}

/** Icon buttons of which one is checked: value, title and icon of each. */
export function Segmented<T extends string>({
  label,
  value,
  options,
  disabled,
  prefix,
  onChange,
}: {
  label: string
  value: T | null
  options: readonly (readonly [T, string, IconName])[]
  disabled?: (v: T) => boolean
  prefix?: string
  onChange: (v: T) => void
}) {
  return (
    <div role="radiogroup" aria-label={label} className="segmented">
      {prefix && <span className="field-label">{prefix}</span>}
      {options.map(([v, title, icon]) => (
        <button key={v} type="button" role="radio" aria-checked={value === v} aria-label={title} title={title} disabled={disabled?.(v)} onClick={() => onChange(v)}>
          <Icon name={icon} />
        </button>
      ))}
    </div>
  )
}

const SIZES = 'satz.panels'
const sizes: Record<string, number> = JSON.parse(localStorage.getItem(SIZES) ?? '{}')
let grab = 0
for (const [side, v] of Object.entries(sizes)) document.documentElement.style.setProperty(`--${side}-size`, `${v}px`)

/** The inner edge of a side panel or the top edge of the swatches, dragged to set its size. */
export function Edge({ side }: { side: 'left' | 'right' | 'swatches' }) {
  const at = (e: ReactPointerEvent<HTMLElement>) => (side === 'left' ? e.clientX : side === 'right' ? innerWidth - e.clientX : innerHeight - e.clientY)
  const size = (el: HTMLElement) => (side === 'swatches' ? el.offsetHeight : el.offsetWidth)
  const [lo, hi] = side === 'swatches' ? [40, innerHeight - 200] : [200, 560]
  return (
    <div
      className="edge"
      aria-hidden="true"
      onPointerDown={(e) => {
        grab = at(e) - size(e.currentTarget.parentElement!)
        e.currentTarget.setPointerCapture(e.pointerId)
        e.preventDefault()
      }}
      onPointerMove={(e) => {
        if (!e.currentTarget.hasPointerCapture(e.pointerId)) return
        const v = Math.round(Math.min(hi, Math.max(lo, at(e) - grab)))
        document.documentElement.style.setProperty(`--${side}-size`, `${v}px`)
        sizes[side] = v
      }}
      onPointerUp={() => localStorage.setItem(SIZES, JSON.stringify(sizes))}
      onDoubleClick={() => {
        document.documentElement.style.removeProperty(`--${side}-size`)
        delete sizes[side]
        localStorage.setItem(SIZES, JSON.stringify(sizes))
      }}
    />
  )
}

/** A checkbox that shows `null`, a mixed value, as indeterminate. */
export function Check({ label, value, set }: { label: string; value: boolean | null; set: (v: boolean) => void }) {
  return (
    <label className="check">
      <input
        type="checkbox"
        checked={value === true}
        ref={(el) => {
          if (el) el.indeterminate = value === null
        }}
        onChange={(e) => set(e.currentTarget.checked)}
      />
      {label}
    </label>
  )
}

const KEYCAP = /^(Ctrl|Alt|Shift|Space|Enter|Esc|Tab|Del|Backspace|Home|End|PgUp|PgDn|F\d+|[^a-z])$/

/** The shortcut `keys` as keycaps, the words of a gesture in it as text. */
export function Keys({ keys }: { keys: string }) {
  return (
    <span className="keys">
      {keys.replace(/(\w)\+/g, '$1 ').split(' ').map((k, i) => (KEYCAP.test(k) ? <kbd key={i}>{keyName(k)}</kbd> : <em key={i}>{k}</em>))}
    </span>
  )
}

export const SIDE_NAMES = ['Top', 'Right', 'Bottom', 'Left']

/** Lengths of the four sides, top first, as a horizontal and a vertical field or, opened, one field per side; `wrap` wraps those. */
export function Sides({
  what,
  values,
  wrap = (_, f) => f,
  opened,
  set,
}: {
  what: string
  values: number[]
  wrap?: (side: number, field: ReactNode) => ReactNode
  /** Keeps the sides open, e.g. while one is bound to a variable. */
  opened?: boolean
  set: (values: number[]) => void
}) {
  const [open, setOpen] = useState(values[0] !== values[2] || values[1] !== values[3])
  const each = open || opened
  return (
    <div className="sides">
      <div className="grid">
        {each
          ? SIDE_NAMES.map((side, i) => (
              <Fragment key={side}>
                {wrap(i, <Field label={side} title={`${side} ${what}`} unit="length" reset={0} value={values[i]} onCommit={(v) => set(values.with(i, v))} />)}
              </Fragment>
            ))
          : ([['↔', 'Horizontal', 1, 3], ['↕', 'Vertical', 0, 2]] as const).map(([label, name, a, b]) => (
              <Field
                key={name}
                label={label}
                title={`${name} ${what}`}
                unit="length"
                reset={0}
                value={values[a] === values[b] ? values[a] : null}
                onCommit={(v) => set(values.with(a, v).with(b, v))}
              />
            ))}
      </div>
      <button
        type="button"
        className="icon-button"
        aria-label={`${what[0].toUpperCase()}${what.slice(1)} of each side`}
        title={`${what[0].toUpperCase()}${what.slice(1)} of each side`}
        aria-pressed={!!each}
        disabled={opened}
        onClick={() => setOpen(!open)}
      >
        <Icon name="sides" />
      </button>
    </div>
  )
}
