import { useEffect, useRef, useState } from 'react'
import { ACTIONS, comboOf, keysOf, type Action } from './commands'
import { Keys } from './controls'
import { Icon } from './icons'
import { setSettings, settings, useSettings } from './settings'

const GESTURES: Record<string, [string, string][]> = {
  Canvas: [
    ['Ctrl click', 'Select a layer inside a group'],
    ['Ctrl Shift click', 'Override a master layer on this page'],
    ['Double-click', 'Edit a text or enter a group'],
    ['Alt drag', 'Resize from the centre'],
    ['Space drag', 'Pan the view'],
    ['Ctrl wheel', 'Zoom the view'],
    ['Ctrl drag', 'Move without snapping'],
    ['Shift drag', 'Move along one axis'],
    ['Drag beside a corner', 'Rotate, with Shift in steps of 15°'],
    ['Enter', 'Edit the points of the selected shape'],
    ['Ctrl D', 'Duplicate a moved copy again as far on'],
    ['Ctrl V', 'Paste text from outside as a new text layer'],
    ['Alt', 'Show the distances to the layer under the pointer'],
    ['← → ↑ ↓', 'Nudge by 0.1 mm, with Shift by 1 mm'],
    ['Drag from a ruler', 'Add a guide; drag it back or press Del to remove it'],
  ],
  Text: [
    ['Enter', 'Edit the selected text'],
    ['Esc', 'Stop editing the text'],
    ['Ctrl Alt Shift N', 'Insert the page number'],
    ['Double-click a port', 'Unthread a text frame'],
    ['lorem Tab', 'Insert placeholder text, lorem50 for 50 words'],
  ],
  Layers: [
    ['↑ ↓', 'Select the previous or next layer'],
    ['→ ←', 'Open or close a group'],
    ['Enter', 'Select the children'],
    ['Shift Enter', 'Select the parent'],
    ['Tab', 'Select the next sibling'],
    ['Shift Tab', 'Select the previous sibling'],
  ],
  Pages: [
    ['Alt ← →', 'Move the page in the overview'],
    ['Double-click a master', 'Edit the master'],
  ],
  Fields: [
    ['Drag the label', 'Change the value'],
    ['Double-click the label', 'Reset to the default'],
    ['↑ ↓', 'Step by 1, with Shift by 10, with Alt by 0.1'],
    ['118/2', 'Type a sum; Enter calculates it'],
    ['12pt', 'Type a unit; Enter converts it'],
  ],
}
const GROUPS = ['Canvas', 'Tools', 'Edit', 'Arrange', 'Align', 'Text', 'Layers', 'Pages', 'View', 'Fields', 'File'].map(
  (g) => [g, [...ACTIONS.filter((a) => a.group === g && a.keys), ...(GESTURES[g] ?? []).map(([keys, title]) => ({ keys, title }))]] as const,
)

const mark = (text: string, q: string) => {
  const at = q ? text.toLowerCase().indexOf(q) : -1
  return at < 0 ? text : [text.slice(0, at), <mark key="m">{text.slice(at, at + q.length)}</mark>, text.slice(at + q.length)]
}

const assign = (a: Action, keys: string | null) => {
  const rest = Object.fromEntries(Object.entries(settings.keys).filter(([t]) => t !== a.title))
  setSettings({ keys: keys && keys !== a.keys ? { ...rest, [a.title]: keys } : rest })
}

/** "?": every shortcut by where it works; a click on the keys of a command records new ones. */
export function Help({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [query, setQuery] = useState('')
  const [recording, setRecording] = useState<string | null>(null)
  const remapped = useSettings().keys
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
        <button type="button" className="button" disabled={!Object.keys(remapped).length} onClick={() => setSettings({ keys: {} })}>
          Reset shortcuts
        </button>
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
                <div key={r.title} className="help-r" data-remapped={r.title in remapped || undefined}>
                  <span>{mark(r.title, q)}</span>
                  {r.title in remapped && (
                    <button type="button" className="icon-button" aria-label={`Reset ${r.title}`} title="Reset to the default" onClick={() => assign(r as Action, null)}>
                      <Icon name="reset" />
                    </button>
                  )}
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
                      {recording === r.title ? <span className="keys-hint">Press a key · Esc cancels</span> : <Keys keys={keys(r)} />}
                    </button>
                  ) : (
                    <Keys keys={r.keys} />
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
