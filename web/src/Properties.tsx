import { useState, type ReactNode } from 'react'
import { MM, bounds, useEditor, type Editor } from './editor'
import type { Command, Node } from './model'

const round = (v: number) => String(Math.round(v * 100) / 100)
const hex = (fill: number) => '#' + (fill >>> 8).toString(16).padStart(6, '0')
const rgba = (hex: string, fill: number) => ((parseInt(hex.slice(1), 16) << 8) | (fill & 0xff)) >>> 0

function Field({
  label,
  value,
  unit,
  onCommit,
  readOnly,
}: {
  label: string
  value: number | null
  unit: string
  onCommit?: (v: number) => void
  readOnly?: boolean
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const commit = () => {
    const v = parseFloat(draft ?? '')
    if (draft !== null && Number.isFinite(v)) onCommit?.(v)
    setDraft(null)
  }
  return (
    <label className="field" title={`${label} in ${unit}`}>
      <span className="field-label">{label}</span>
      <input
        name={label.toLowerCase()}
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

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="section" aria-label={title}>
      <h3>{title}</h3>
      {children}
    </section>
  )
}

export function Properties({ editor, onExport }: { editor: Editor; onExport: () => void }) {
  const page = useEditor(editor, (e) => e.page)
  const selection = useEditor(editor, (e) => e.selection)
  const nodes = selection.flatMap((id) => editor.nodes.get(id)?.node ?? [])

  const same = <T,>(get: (n: Node) => T | undefined) => {
    const values = nodes.map(get)
    return values.every((v) => v === values[0]) ? (values[0] ?? null) : null
  }
  const each = (cmd: (n: Node) => Command | undefined) => {
    editor.apply({ type: 'beginUndoGroup' })
    for (const n of nodes) {
      const c = cmd(n)
      if (c) editor.apply(c)
    }
    editor.apply({ type: 'endUndoGroup' })
  }
  const frame = (key: 'x' | 'y' | 'w' | 'h') => (v: number) =>
    each((n) => ({ type: 'setFrame', id: n.id, x: n.x, y: n.y, w: n.w, h: n.h, [key]: Math.max(key === 'w' || key === 'h' ? 0 : -Infinity, v * MM) }))

  const one = nodes.length === 1 ? nodes[0] : undefined
  const fill = one && 'fill' in one ? one.fill : undefined
  const box = nodes.length ? bounds(nodes) : undefined

  return (
    <aside className="panel properties" aria-label="Properties">
      <header className="panel-header">
        <h2>{one ? one.name : nodes.length ? `${nodes.length} layers` : 'Page'}</h2>
        <button type="button" className="primary" onClick={onExport} title="Export PDF (Ctrl+Shift+E)">
          Export PDF
        </button>
      </header>
      {!box && (
        <Section title="Page">
          <div className="grid">
            <Field label="W" value={page.width / MM} unit="mm" readOnly />
            <Field label="H" value={page.height / MM} unit="mm" readOnly />
            <Field label="Bleed" value={page.bleed / MM} unit="mm" readOnly />
          </div>
        </Section>
      )}
      {box && (
        <Section title="Layout">
          <div className="grid">
            <Field label="X" value={nodes.length > 1 ? same((n) => n.x / MM) : box.x / MM} unit="mm" onCommit={frame('x')} />
            <Field label="Y" value={nodes.length > 1 ? same((n) => n.y / MM) : box.y / MM} unit="mm" onCommit={frame('y')} />
            <Field label="W" value={same((n) => n.w / MM)} unit="mm" onCommit={frame('w')} />
            <Field label="H" value={same((n) => n.h / MM)} unit="mm" onCommit={frame('h')} />
          </div>
          {one?.kind === 'frame' && (
            <label className="check">
              <input
                type="checkbox"
                checked={one.clip}
                onChange={(e) => editor.apply({ type: 'set', id: one.id, clip: e.currentTarget.checked })}
              />
              Clip content
            </label>
          )}
        </Section>
      )}
      {one && fill !== undefined && (
        <Section title="Fill">
          <div className="fill">
            <input
              type="color"
              aria-label="Fill color"
              value={hex(fill)}
              onChange={(e) => editor.apply({ type: 'set', id: one.id, fill: rgba(e.currentTarget.value, fill || 0xff) })}
            />
            <span className="hex">{hex(fill).slice(1).toUpperCase()}</span>
            <span className="alpha">{Math.round(((fill & 0xff) / 255) * 100)}%</span>
          </div>
        </Section>
      )}
      {one?.kind === 'text' && (
        <Section title="Text">
          <div className="grid">
            <Field
              label="Size"
              value={one.size}
              unit="pt"
              onCommit={(size) => size > 0 && editor.apply({ type: 'set', id: one.id, size })}
            />
          </div>
          <textarea
            className="content"
            aria-label="Text content"
            rows={6}
            defaultValue={one.text}
            key={one.id}
            onBlur={(e) => {
              if (e.currentTarget.value !== one.text) editor.apply({ type: 'setText', id: one.id, text: e.currentTarget.value })
            }}
          />
        </Section>
      )}
    </aside>
  )
}
