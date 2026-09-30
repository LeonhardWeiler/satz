import { useRef, useState, type KeyboardEvent, type MouseEvent, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu } from './ContextMenu'
import { MM } from './editor'
import { evalExpr, step } from './field'
import { Icon, type IconName } from './icons'

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
/** Points per unit of the lengths that fields show in another unit than they are given in. */
const PT: Record<string, number> = { mm: MM }

/**
 * A number input that takes expressions with units. ↑/↓ step it, Shift by 10 and Alt by 0.1;
 * dragging its label scrubs it; invalid input shakes and keeps the value. Values stay within `min`
 * and `max`, in `unit`.
 */
export function Field({
  label,
  value,
  unit,
  onCommit,
  readOnly,
  zero,
  int,
  min = 0,
  max = Infinity,
  title = typeof label === 'string' ? `${label} in ${unit}` : unit,
}: {
  label: ReactNode
  /** In pt for lengths, shown in `unit`. */
  value: number | null
  unit: string
  onCommit?: (v: number) => void
  readOnly?: boolean
  /** Shown for 0 and accepted as input for it, e.g. "Auto". */
  zero?: string
  /** Whole numbers with − and + buttons. */
  int?: boolean
  min?: number
  max?: number
  title?: string
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const [bad, setBad] = useState(false)
  const [scrub, setScrub] = useState(false)
  const per = PT[unit] ?? 1
  const shown = value === null ? null : round(value / per, unit)
  const put = (v: number) => {
    const c = Math.min(max, Math.max(min, v))
    onCommit?.((int ? Math.round(c) : round(c, unit)) * per)
  }
  const edit = !readOnly && onCommit && shown !== null
  const cur = shown ?? 0
  const commit = () => {
    setDraft(null)
    if (draft === null) return
    const v = zero && draft.trim().toLowerCase() === zero.toLowerCase() ? 0 : evalExpr(draft, cur, unit)
    if (v === null) setBad(true)
    else put(v)
  }
  const bump = (d: number) => edit && put(cur + d)
  return (
    <label className={`field${int ? ' int' : ''}${bad ? ' bad' : ''}${scrub ? ' scrub' : ''}`} title={title} onAnimationEnd={() => setBad(false)}>
      {label && (
        <span
          className="field-label"
          onClick={(e) => e.preventDefault()}
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
        name={title.toLowerCase().replaceAll(' ', '-')}
        aria-label={title}
        inputMode="decimal"
        autoComplete="off"
        spellCheck={false}
        readOnly={readOnly}
        value={draft ?? (shown === null ? 'Mixed' : shown === 0 && zero ? zero : String(shown))}
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
export function NameInput({ label, value, autoFocus, onCommit }: { label: string; value: string; autoFocus?: boolean; onCommit: (v: string) => void }) {
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
        if (v && v !== value) onCommit(v)
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
  onChange,
}: {
  label: string
  value: T | null
  options: Record<T, ReactNode>
  disabled?: T[]
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
        onClick={open}
        onKeyDown={(e) => {
          if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
          e.preventDefault()
          open(e)
        }}
      >
        <span>{value === null ? 'Mixed' : options[value]}</span>
        <Icon name="chevron" />
      </button>
      {host &&
        createPortal(
          <ContextMenu
            anchor={() => ref.current!.getBoundingClientRect()}
            side="bottom"
            label={label}
            onClose={() => setHost(null)}
            items={(Object.keys(options) as T[]).map((v) => [options[v], () => onChange(v), !disabled.includes(v), v === value])}
          />,
          host,
        )}
    </>
  )
}

export function Section({ title, onAdd, children }: { title: string; onAdd?: (e: MouseEvent<HTMLButtonElement>) => void; children?: ReactNode }) {
  const add = `Add ${title.toLowerCase().replace(/s$/, '')}`
  return (
    <section className="section" aria-label={title}>
      <header className="section-header">
        <h3>{title}</h3>
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
  onChange,
}: {
  label: string
  value: T | null
  options: readonly (readonly [T, string, IconName])[]
  disabled?: (v: T) => boolean
  onChange: (v: T) => void
}) {
  return (
    <div role="radiogroup" aria-label={label} className="segmented">
      {options.map(([v, title, icon]) => (
        <button key={v} type="button" role="radio" aria-checked={value === v} aria-label={title} title={title} disabled={disabled?.(v)} onClick={() => onChange(v)}>
          <Icon name={icon} />
        </button>
      ))}
    </div>
  )
}
