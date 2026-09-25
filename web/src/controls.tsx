import { useState, type ReactNode } from 'react'
import { Icon } from './icons'

const round = (v: number, unit: string) => (unit === '%' ? Math.round(v) : Math.round(v * 100) / 100)

export function Field({
  label,
  value,
  unit,
  onCommit,
  readOnly,
  title = `${label} in ${unit}`,
}: {
  label: string
  value: number | null
  unit: string
  onCommit?: (v: number) => void
  readOnly?: boolean
  title?: string
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const commit = () => {
    const v = parseFloat(draft ?? '')
    setDraft(null)
    if (draft === null || !Number.isFinite(v)) return
    try {
      onCommit?.(round(v, unit))
    } catch (e) {
      console.warn(e)
    }
  }
  return (
    <label className="field" title={title}>
      {label && <span className="field-label">{label}</span>}
      <input
        name={title.toLowerCase().replaceAll(' ', '-')}
        aria-label={title}
        inputMode="decimal"
        autoComplete="off"
        spellCheck={false}
        readOnly={readOnly}
        value={draft ?? (value === null ? 'Mixed' : String(round(value, unit)))}
        onChange={(e) => setDraft(e.currentTarget.value)}
        onFocus={(e) => e.currentTarget.select()}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') e.currentTarget.blur()
          if (e.key === 'Escape') {
            setDraft(null)
            e.currentTarget.blur()
          }
        }}
      />
      {(value !== null || draft !== null) && <span className="field-unit">{unit}</span>}
    </label>
  )
}

/** A text input that commits a changed, non-empty value on blur or Enter. */
export function NameInput({ label, value, onCommit }: { label: string; value: string; onCommit: (v: string) => void }) {
  return (
    <input
      key={value}
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
        if (e.key === 'Enter') e.currentTarget.blur()
      }}
    />
  )
}

/** The next free `prefix n` among `names`. */
export const nextName = (prefix: string, names: string[]) =>
  `${prefix} ${Math.max(0, ...names.map((n) => (n.startsWith(`${prefix} `) ? Number(n.slice(prefix.length + 1)) || 0 : 0))) + 1}`

export function Select<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: T
  options: Record<T, string>
  onChange: (v: T) => void
}) {
  return (
    <select className="select" aria-label={label} title={label} value={value} onChange={(e) => onChange(e.currentTarget.value as T)}>
      {Object.entries<string>(options).map(([v, text]) => (
        <option key={v} value={v}>
          {text}
        </option>
      ))}
    </select>
  )
}

export function Section({
  title,
  onAdd,
  children,
}: {
  title: string
  onAdd?: () => void
  children?: ReactNode
}) {
  const add = `Add ${title.toLowerCase().replace(/s$/, '')}`
  return (
    <section className="section" aria-label={title}>
      <header className="section-header">
        <h3>{title}</h3>
        {onAdd && (
          <button type="button" className="icon-button" aria-label={add} title={add} onClick={onAdd}>
            <Icon name="plus" size={16} />
          </button>
        )}
      </header>
      {children}
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
        <Icon name={visible ? 'eye' : 'eyeOff'} size={16} />
      </button>
      <button type="button" className="icon-button" aria-label={`Remove ${what}`} title={`Remove ${what}`} onClick={onRemove}>
        <Icon name="minus" size={16} />
      </button>
    </>
  )
}
