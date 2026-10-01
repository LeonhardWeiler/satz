import { useEffect, useRef, useState } from 'react'
import { ACTIONS, comboOf, keyLabel, keysOf, type Action } from './commands'
import { Icon } from './icons'
import { setSettings, settings, useSettings } from './settings'

const GESTURES: Record<string, [string, string][]> = {
  Canvas: [['Ctrl click', 'Select inside a group'], ['Ctrl Shift click', 'Override a master layer'], ['Double-click', 'Edit text, enter a group'], ['Alt drag', 'Resize from the centre'], ['Space drag', 'Pan'], ['Ctrl wheel', 'Zoom'], ['Ctrl drag', 'No snapping'], ['Shift drag', 'Lock to an axis'], ['Drag beside a corner', 'Turn, Shift by 15°'], ['Enter on a shape', 'Edit its points'], ['Ctrl D on a moved copy', 'Copy again as far on'], ['Ctrl V of outside text', 'New text layer'], ['Ctrl V away from the copy', 'Paste under the pointer'], ['Alt', 'Distances to the layer under the pointer'], ['Arrows', 'Nudge 0.1 mm, Shift 1 mm']],
  Text: [['Enter', 'Edit the selected text'], ['Esc', 'Stop editing'], ['Ctrl Alt Shift N', 'Insert the page number'], ['Double-click a port', 'Unthread'], ['lorem Tab', 'Lorem ipsum, lorem50 for 50 words']],
  Layers: [['↑ ↓', 'Previous, next layer'], ['→ ←', 'Open, close a group'], ['Enter', 'Select children'], ['Shift Enter', 'Select parent'], ['Tab', 'Next sibling'], ['Shift Tab', 'Previous sibling']],
  Pages: [['Alt ← →', 'Move the page in the overview'], ['Double-click master', 'Edit master']],
  Fields: [['Drag the label', 'Change the value'], ['Double-click the label', 'Default value'], ['↑ ↓', '1, Shift 10, Alt 0.1'], ['118/2', 'Calculates on Enter'], ['12pt', 'Converts the unit']],
}
const GROUPS = ['Canvas', 'Tools', 'Edit', 'Arrange', 'Align', 'Text', 'Layers', 'Pages', 'View', 'Fields', 'File'].map(
  (g) => [g, [...ACTIONS.filter((a) => a.group === g && a.keys), ...(GESTURES[g] ?? []).map(([keys, title]) => ({ keys, title }))]] as const,
)

const assign = (a: Action, keys: string | null) => {
  const rest = Object.fromEntries(Object.entries(settings.keys).filter(([t]) => t !== a.title))
  setSettings({ keys: keys && keys !== a.keys ? { ...rest, [a.title]: keys } : rest })
}

/** "?": every shortcut by where it works; a click on the keys of a command records new ones. */
export function Help({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [query, setQuery] = useState('')
  const [recording, setRecording] = useState<string | null>(null)
  const remapped = Object.keys(useSettings().keys).length > 0
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  const q = query.trim().toLowerCase()
  const keys = (r: Action | { keys: string }) => ('group' in r ? keysOf(r) : r.keys)
  const groups = GROUPS.map(([g, rows]) => [g, rows.filter((r) => `${g} ${r.title} ${keys(r)}`.toLowerCase().includes(q))] as const).filter(([, rows]) => rows.length)
  return (
    <dialog ref={ref} className="help" aria-label="Keyboard shortcuts" onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <header className="help-head">
        <h2>Keyboard shortcuts</h2>
        {remapped && (
          <button type="button" className="button" onClick={() => setSettings({ keys: {} })}>
            Reset shortcuts
          </button>
        )}
        <input className="help-search" aria-label="Search shortcuts" placeholder="Search" autoFocus value={query} onChange={(e) => setQuery(e.target.value)} />
        <button type="button" className="icon-button" aria-label="Close" onClick={() => ref.current!.close()}>
          <Icon name="close" />
        </button>
      </header>
      <div className="help-body">
        <div className="help-grid">
          {groups.map(([group, rows]) => (
            <section key={group} aria-label={group}>
              <h3>{group}</h3>
              {rows.map((r) => (
                <div key={r.title} className="help-r">
                  <span>{r.title}</span>
                  {'group' in r ? (
                    <button
                      type="button"
                      className="help-keys"
                      aria-label={`Shortcut for ${r.title}`}
                      title="Click and press new keys, Backspace for the default"
                      data-recording={recording === r.title || undefined}
                      onClick={() => setRecording(r.title)}
                      onBlur={() => setRecording(null)}
                      onKeyDown={(e) => {
                        if (recording !== r.title) return
                        e.preventDefault()
                        e.stopPropagation()
                        const combo = comboOf(e.nativeEvent)
                        if (!combo) return
                        if (combo !== 'Escape') assign(r, combo === 'Backspace' ? null : combo)
                        setRecording(null)
                      }}
                    >
                      <kbd>{recording === r.title ? 'Press keys' : keyLabel(keys(r))}</kbd>
                    </button>
                  ) : (
                    <kbd>{keyLabel(r.keys)}</kbd>
                  )}
                </div>
              ))}
            </section>
          ))}
        </div>
        {!groups.length && <p className="help-empty">No shortcut matches.</p>}
      </div>
    </dialog>
  )
}
