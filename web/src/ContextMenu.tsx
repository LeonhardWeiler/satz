import { useEffect, useRef, useState, type ReactNode } from 'react'
import { roam } from './controls'
import { Icon } from './icons'
import { Popover, type Anchor } from './Popover'

/** A menu beside `anchor` that closes on a pick, Escape or a click elsewhere; an item with a fourth value is a radio item, a fifth its shortcut, `null` a separator. */
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
  items: ([ReactNode, () => void, boolean, boolean?, string?] | null)[]
  onClose: () => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [back] = useState(() => document.activeElement as HTMLElement | null)
  useEffect(() => {
    if (!ref.current!.contains(document.activeElement)) ref.current!.focus()
    return () => back?.focus({ preventScroll: true })
  }, [back])
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
        {items.map((item, i) => {
          if (!item) return <hr key={i} className="menu-sep" />
          const [label, run, enabled, checked, keys] = item
          return (
          <button
            key={i}
            type="button"
            role={checked === undefined ? 'menuitem' : 'menuitemradio'}
            aria-checked={checked}
            className="menu-item"
            disabled={!enabled}
            autoFocus={i === Math.max(0, items.findIndex((it) => it?.[3]))}
            onClick={() => {
              onClose()
              run()
            }}
          >
            <span>{checked && <Icon name="check" />}</span>
            <span>{label}</span>
            {keys && <kbd>{keys}</kbd>}
          </button>
          )
        })}
      </Popover>
    </div>
  )
}
