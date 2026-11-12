import { useLayoutEffect, useRef, type HTMLAttributes, type ReactNode, type RefObject } from 'react'
import { createPortal } from 'react-dom'

export type Anchor = () => DOMRect

const GAP = 8
const MARGIN = 8

/** Start of a `size` long box beside [lo, hi], before or `after` it, flipped when it does not fit. */
function flip(lo: number, hi: number, size: number, room: number, after: boolean) {
  const before = lo - GAP - size
  const fitsBefore = before >= MARGIN
  const fitsAfter = hi + GAP + size <= room - MARGIN
  return (after ? !fitsAfter && fitsBefore : fitsBefore || !fitsAfter) ? before : hi + GAP
}

const shift = (v: number, size: number, room: number) => Math.max(MARGIN, Math.min(v, room - MARGIN - size))

/** Floats beside `anchor` on `side`, flipped to the other side and shifted to stay inside the window; above or below it starts at its left edge or ends at its right one. */
export function Popover({
  anchor,
  side,
  ref: outer,
  ...props
}: {
  anchor: Anchor
  side: 'left' | 'right' | 'top' | 'bottom'
  ref?: RefObject<HTMLDivElement | null>
} & HTMLAttributes<HTMLDivElement>) {
  const own = useRef<HTMLDivElement>(null)
  const ref = outer ?? own
  const latest = useRef(anchor)
  useLayoutEffect(() => {
    latest.current = anchor
  })
  useLayoutEffect(() => {
    const el = ref.current!
    const place = () => {
      const a = latest.current()
      const { offsetWidth: w, offsetHeight: h } = el
      const [vw, vh] = [window.innerWidth, window.innerHeight]
      const [x, y] =
        side === 'top' || side === 'bottom' ? [a.left + w <= vw - MARGIN ? a.left : a.right - w, flip(a.top, a.bottom, h, vh, side === 'bottom')] : [flip(a.left, a.right, w, vw, side === 'right'), a.top]
      el.style.left = `${shift(x, w, vw)}px`
      el.style.top = `${shift(y, h, vh)}px`
    }
    place()
    const observer = new ResizeObserver(place)
    observer.observe(el)
    window.addEventListener('resize', place)
    window.addEventListener('scroll', place, true)
    return () => {
      observer.disconnect()
      window.removeEventListener('resize', place)
      window.removeEventListener('scroll', place, true)
    }
  }, [side, ref])
  return <div ref={ref} {...props} className={`popover ${props.className ?? ''}`} />
}

/** The row of `el` stretched across its panel, so that a popover on its side docks to the panel's edge. */
export const docked =
  (el: Element): Anchor =>
  () => {
    const b = el.getBoundingClientRect()
    const panel = el.closest('.panel, .left')?.getBoundingClientRect() ?? b
    return new DOMRect(panel.x, b.y, panel.width, b.height)
  }

/** A dialog docked beside the panel of `at` that closes on Escape or a click outside. */
export function Dialog({ label, at, className, onClose, children }: { label: string; at: Element; className?: string; onClose: () => void; children: ReactNode }) {
  return createPortal(
    <div className="menu-backdrop" onPointerDown={onClose}>
      <Popover
        anchor={docked(at)}
        side="left"
        className={`picker ${className ?? 'options'}`}
        role="dialog"
        aria-label={label}
        tabIndex={-1}
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key !== 'Escape') return
          e.stopPropagation()
          onClose()
        }}
      >
        {children}
      </Popover>
    </div>,
    document.body,
  )
}
