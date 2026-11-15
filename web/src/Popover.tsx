import { useLayoutEffect, useRef, type HTMLAttributes, type ReactNode, type RefObject } from 'react'
import { createPortal } from 'react-dom'

export type Anchor = () => DOMRect

const GAP = 8
const MARGIN = 8

function flip(lo: number, hi: number, size: number, room: number, after: boolean) {
  const before = lo - GAP - size
  const fitsBefore = before >= MARGIN
  const fitsAfter = hi + GAP + size <= room - MARGIN
  return (after ? !fitsAfter && fitsBefore : fitsBefore || !fitsAfter) ? before : hi + GAP
}

const shift = (v: number, size: number, room: number) => Math.max(MARGIN, Math.min(v, room - MARGIN - size))

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

export const docked =
  (el: Element): Anchor =>
  () => {
    const b = el.getBoundingClientRect()
    const panel = el.closest('.panel, .left')?.getBoundingClientRect() ?? b
    return new DOMRect(panel.x, b.y, panel.width, b.height)
  }

export function Dialog({ label, at, className, onClose, children }: { label: string; at: Element; className?: string; onClose: () => void; children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null)
  useLayoutEffect(() => void dispatchEvent(new CustomEvent('cover', { detail: ref.current!.getBoundingClientRect() })), [])
  return createPortal(
    <div className="menu-backdrop" onPointerDown={onClose}>
      <Popover
        ref={ref}
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
        <header className="picker-header">
          <h2>{label}</h2>
        </header>
        {children}
      </Popover>
    </div>,
    document.body,
  )
}
