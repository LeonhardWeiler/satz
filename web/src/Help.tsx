import { useEffect, useRef } from 'react'
import { ACTIONS } from './commands'
import { Icon } from './icons'

const GROUPS: [string, [string, string][]][] = [
  ['Canvas', [['Ctrl click', 'Select inside a group'], ['Ctrl Shift click', 'Override a master layer'], ['Double-click', 'Edit text, enter a group'], ['Alt drag', 'Resize from the centre'], ['Space drag', 'Pan'], ['Ctrl wheel', 'Zoom'], ['Ctrl drag', 'No snapping'], ['Shift drag', 'Lock to an axis'], ['Alt', 'Distances to the layer under the pointer']]],
  ['Layers', [['↑ ↓', 'Previous, next layer'], ['→ ←', 'Open, close a group'], ['Enter', 'Select children'], ['Shift Enter', 'Select parent'], ['Tab', 'Next sibling'], ['Shift Tab', 'Previous sibling'], ['F2', 'Rename'], ['Arrows on canvas', 'Nudge 1 mm, Shift 10 mm']]],
  ['Fields', [['Drag the label', 'Change the value'], ['↑ ↓', '1, Shift 10, Alt 0.1'], ['118/2', 'Calculates on Enter'], ['12pt', 'Converts the unit']]],
  ['Text', [['Enter', 'Edit the selected text'], ['Esc', 'Stop editing'], ['Ctrl Alt Shift N', 'Insert the page number'], ['Double-click a port', 'Unthread']]],
  ['Pages', [['PgUp PgDn', 'Previous, next spread'], ['Home End', 'First, last spread'], ['.', 'Page overview'], ['Alt ← →', 'Move the page in the overview'], ['Double-click master', 'Edit master']]],
  ['Tools', ACTIONS.filter((a) => a.tool).map((a) => [a.keys, a.title])],
  ['Commands', ACTIONS.filter((a) => a.keys && !a.tool && !['PgUp', 'PgDn', 'Home', 'End', '.', 'F2'].includes(a.keys)).map((a) => [a.keys, a.title])],
]

/** "?": every shortcut by where it works. */
export function Help({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  return (
    <dialog ref={ref} className="help" aria-label="Keyboard shortcuts" onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <header className="help-head">
        <h2>Keyboard shortcuts</h2>
        <button type="button" className="icon-button" aria-label="Close" onClick={() => ref.current!.close()}>
          <Icon name="close" />
        </button>
      </header>
      <div className="help-grid">
        {GROUPS.map(([group, rows]) => (
          <section key={group} aria-label={group}>
            <h3>{group}</h3>
            {rows.map(([keys, title]) => (
              <div key={title} className="help-r">
                <kbd>{keys}</kbd>
                <span>{title}</span>
              </div>
            ))}
          </section>
        ))}
      </div>
    </dialog>
  )
}
