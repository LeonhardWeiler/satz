import { useEffect, useRef, useState } from 'react'
import { ALIGNS } from './align'
import { ACTIONS, keyLabel } from './commands'
import { Icon } from './icons'

const GESTURES: Record<string, [string, string][]> = {
  Canvas: [['Ctrl click', 'Select inside a group'], ['Ctrl Shift click', 'Override a master layer'], ['Double-click', 'Edit text, enter a group'], ['Alt drag', 'Resize from the centre'], ['Space drag', 'Pan'], ['Ctrl wheel', 'Zoom'], ['Ctrl drag', 'No snapping'], ['Shift drag', 'Lock to an axis'], ['Drag beside a corner', 'Turn, Shift by 15°'], ['Enter on a shape', 'Edit its points'], ['Ctrl D on a moved copy', 'Copy again as far on'], ['Ctrl V of outside text', 'New text layer'], ['Alt', 'Distances to the layer under the pointer'], ['Arrows', 'Nudge 0.1 mm, Shift 1 mm']],
  Align: ALIGNS.map(([, title, , keys]) => [keys.replaceAll('+', ' '), title]),
  Text: [['Enter', 'Edit the selected text'], ['Esc', 'Stop editing'], ['Ctrl Alt Shift N', 'Insert the page number'], ['Double-click a port', 'Unthread'], ['lorem Tab', 'Lorem ipsum, lorem50 for 50 words']],
  Layers: [['↑ ↓', 'Previous, next layer'], ['→ ←', 'Open, close a group'], ['Enter', 'Select children'], ['Shift Enter', 'Select parent'], ['Tab', 'Next sibling'], ['Shift Tab', 'Previous sibling']],
  Pages: [['Alt ← →', 'Move the page in the overview'], ['Double-click master', 'Edit master']],
  Fields: [['Drag the label', 'Change the value'], ['Double-click the label', 'Default value'], ['↑ ↓', '1, Shift 10, Alt 0.1'], ['118/2', 'Calculates on Enter'], ['12pt', 'Converts the unit']],
}
const GROUPS = ['Canvas', 'Tools', 'Edit', 'Arrange', 'Align', 'Text', 'Layers', 'Pages', 'View', 'Fields', 'File'].map(
  (g) => [g, [...ACTIONS.filter((a) => a.group === g && a.keys).map((a) => [a.keys, a.title]), ...(GESTURES[g] ?? [])]] as const,
)

/** "?": every shortcut by where it works. */
export function Help({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [query, setQuery] = useState('')
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  const q = query.trim().toLowerCase()
  const groups = GROUPS.map(([g, rows]) => [g, rows.filter(([keys, title]) => `${g} ${title} ${keys}`.toLowerCase().includes(q))] as const).filter(([, rows]) => rows.length)
  return (
    <dialog ref={ref} className="help" aria-label="Keyboard shortcuts" onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <header className="help-head">
        <h2>Keyboard shortcuts</h2>
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
              {rows.map(([keys, title]) => (
                <div key={title} className="help-r">
                  <span>{title}</span>
                  <kbd>{keyLabel(keys)}</kbd>
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
