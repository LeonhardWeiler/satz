import { Popover } from './Popover'

/** A context menu at the pointer that closes on a pick, Escape or a click elsewhere. */
export function ContextMenu({
  menu,
  label,
  items,
  onClose,
}: {
  menu: { x: number; y: number }
  label: string
  items: [string, () => void, boolean][]
  onClose: () => void
}) {
  return (
    <div className="menu-backdrop" onPointerDown={onClose} onContextMenu={(e) => e.preventDefault()}>
      <Popover
        anchor={() => new DOMRect(menu.x, menu.y, 0, 0)}
        side="right"
        className="menu"
        role="menu"
        aria-label={label}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === 'Escape') onClose()
        }}
      >
        {items.map(([label, run, enabled], i) => (
          <button
            key={label}
            type="button"
            role="menuitem"
            className="menu-item"
            disabled={!enabled}
            autoFocus={i === 0}
            onClick={() => {
              onClose()
              run()
            }}
          >
            <span />
            <span>{label}</span>
          </button>
        ))}
      </Popover>
    </div>
  )
}
