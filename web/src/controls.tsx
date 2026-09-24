import { useState, type ReactNode } from 'react'
import { Icon } from './icons'

const round = (v: number) => String(Math.round(v * 100) / 100)

export const hex = (color: number) => '#' + (color >>> 8).toString(16).padStart(6, '0')
export const withHex = (color: number, hex: string) => ((parseInt(hex.slice(1), 16) << 8) | (color & 0xff)) >>> 0
export const alpha = (color: number) => ((color & 0xff) / 255) * 100
export const withAlpha = (color: number, percent: number) =>
  ((color & ~0xff) | Math.round(Math.min(100, Math.max(0, percent)) * 2.55)) >>> 0

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
    if (draft !== null && Number.isFinite(v)) onCommit?.(v)
    setDraft(null)
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
        value={draft ?? (value === null ? 'Mixed' : round(value))}
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
      <span className="field-unit">{unit}</span>
    </label>
  )
}

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

export function Section({ title, onAdd, children }: { title: string; onAdd?: () => void; children?: ReactNode }) {
  return (
    <section className="section" aria-label={title}>
      <header className="section-header">
        <h3>{title}</h3>
        {onAdd && (
          <button type="button" className="icon-button" aria-label={`Add ${title.toLowerCase()}`} title={`Add ${title.toLowerCase()}`} onClick={onAdd}>
            <Icon name="plus" size={16} />
          </button>
        )}
      </header>
      {children}
    </section>
  )
}

export function Swatch({ label, color, onChange }: { label: string; color: number; onChange: (c: number) => void }) {
  return (
    <input
      type="color"
      className="swatch"
      aria-label={label}
      title={label}
      value={hex(color)}
      onChange={(e) => onChange(withHex(color, e.currentTarget.value))}
    />
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
