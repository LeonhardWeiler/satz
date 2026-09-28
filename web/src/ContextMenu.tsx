import { useEffect, useRef, type ReactNode } from 'react'
import { roam } from './controls'
import { Popover } from './Popover'

/** A menu at the pointer that closes on a pick, Escape or a click elsewhere; an item with a fourth value is a radio item. */
export function ContextMenu({
  menu,
  label,
  items,
  onClose,
}: {
  menu: { x: number; y: number }
  label: string
  items: [ReactNode, () => void, boolean, boolean?][]
  onClose: () => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!ref.current!.contains(document.activeElement)) ref.current!.focus()
  }, [])
  return (
    <div className="menu-backdrop" onPointerDown={onClose} onContextMenu={(e) => e.preventDefault()}>
      <Popover
        anchor={() => new DOMRect(menu.x, menu.y, 0, 0)}
        side="right"
        ref={ref}
        tabIndex={-1}
        className="menu"
        role="menu"
        aria-label={label}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === 'Escape') onClose()
          else roam(e, [...e.currentTarget.querySelectorAll('[role^=menuitem]:not(:disabled)')])
        }}
      >
        {items.map(([label, run, enabled, checked], i) => (
          <button
            key={i}
            type="button"
            role={checked === undefined ? 'menuitem' : 'menuitemradio'}
            aria-checked={checked}
            className="menu-item"
            disabled={!enabled}
            autoFocus={i === Math.max(0, items.findIndex((it) => it[3]))}
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
