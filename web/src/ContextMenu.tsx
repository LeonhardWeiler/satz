import { useEffect, useRef, type ReactNode } from 'react'
import { roam } from './controls'
import { Icon } from './icons'
import { Popover, type Anchor } from './Popover'

/** A menu beside `anchor` that closes on a pick, Escape or a click elsewhere; an item with a fourth value is a radio item. */
export function ContextMenu({
  anchor,
  side = 'right',
  label,
  items,
  onClose,
}: {
  anchor: Anchor
  side?: 'right' | 'bottom'
  label: string
  items: [ReactNode, () => void, boolean, boolean?][]
  onClose: () => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const back = document.activeElement as HTMLElement | null
    if (!ref.current!.contains(back)) ref.current!.focus()
    return () => back?.focus({ preventScroll: true })
  }, [])
  return (
    <div className="menu-backdrop" onPointerDown={onClose} onContextMenu={(e) => e.preventDefault()}>
      <Popover
        anchor={anchor}
        side={side}
        ref={ref}
        tabIndex={-1}
        className="menu"
        role="menu"
        aria-label={label}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault()
            e.stopPropagation()
            onClose()
          }
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
            <span>{checked && <Icon name="check" />}</span>
            <span>{label}</span>
          </button>
        ))}
      </Popover>
    </div>
  )
}
