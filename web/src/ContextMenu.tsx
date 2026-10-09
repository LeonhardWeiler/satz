import { useEffect, useRef, useState, type ReactNode } from 'react'
import { Keys, roam } from './controls'
import { Icon } from './icons'
import { Popover, type Anchor } from './Popover'

/** A menu beside `anchor` that closes on a pick, Escape or a click elsewhere; an item with a fourth value is a radio item, a fifth its shortcut, `null` a separator; a letter moves to the next item it starts, or with `search` filters the items. */
export function ContextMenu({
  anchor,
  side = 'right',
  label,
  items,
  search,
  onClose,
}: {
  anchor: Anchor
  side?: 'right' | 'bottom'
  label: string
  items: ([ReactNode, () => void, boolean, boolean?, string?] | null)[]
  search?: boolean
  onClose: () => void
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [query, setQuery] = useState('')
  const q = query.trim().toLowerCase()
  const radio = items.some((it) => it?.[3] !== undefined)
  const shown = q ? items.filter((it) => it && String(it[0]).toLowerCase().includes(q)) : items
  const [back] = useState(() => document.activeElement as HTMLElement | null)
  useEffect(() => {
    if (!ref.current!.contains(document.activeElement)) ref.current!.focus()
    return () => {
      if (document.activeElement === document.body) back?.focus({ preventScroll: true })
    }
  }, [back])
  return (
    <div className="menu-backdrop" onPointerDown={onClose} onContextMenu={(e) => e.preventDefault()}>
      <Popover
        anchor={anchor}
        side={side}
        ref={ref}
        tabIndex={-1}
        className={radio ? 'menu' : 'menu plain'}
        style={side === 'bottom' ? { minWidth: anchor().width } : undefined}
        role="menu"
        aria-label={label}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault()
            e.stopPropagation()
            onClose()
          }
          else {
            const items = [...e.currentTarget.querySelectorAll<HTMLElement>('.menu-search, [role^=menuitem]:not(:disabled)')]
            if (roam(e, items) || e.key.length !== 1 || e.target instanceof HTMLInputElement) return
            const from = items.indexOf(e.target as HTMLElement) + 1
            const key = e.key.toLowerCase()
            ;[...items.slice(from), ...items.slice(0, from)].find((el) => el.textContent!.toLowerCase().startsWith(key))?.focus()
          }
        }}
      >
        {search && (
          <input
            className="menu-search"
            aria-label={`Search ${label}`}
            placeholder="Search"
            spellCheck={false}
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== 'Enter') return
              e.preventDefault()
              e.currentTarget.parentElement!.querySelector<HTMLElement>('[role^=menuitem]:not(:disabled)')?.click()
            }}
          />
        )}
        {shown.map((item, i) => {
          if (!item) return <hr key={i} className="menu-sep" />
          const [label, run, enabled, checked, keys] = item
          return (
          <button
            key={i}
            type="button"
            role={checked === undefined ? 'menuitem' : 'menuitemradio'}
            aria-checked={checked}
            className={String(label).startsWith('Delete') ? 'menu-item danger' : 'menu-item'}
            disabled={!enabled}
            autoFocus={!search && i === Math.max(0, items.findIndex((it) => it?.[3]))}
            onClick={() => {
              onClose()
              run()
            }}
          >
            {radio && <span>{checked && <Icon name="check" />}</span>}
            <span>{label}</span>
            {keys && <Keys keys={keys} />}
          </button>
          )
        })}
      </Popover>
    </div>
  )
}
