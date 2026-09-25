import { useLayoutEffect, useRef, type HTMLAttributes, type RefObject } from 'react'

export type Anchor = () => DOMRect

const GAP = 8
const MARGIN = 8

/** Start of a `size` long box beside [lo, hi]: before it, or after it when it does not fit before. */
function flip(lo: number, hi: number, size: number, room: number) {
  const before = lo - GAP - size
  return before >= MARGIN || hi + GAP + size > room - MARGIN ? before : hi + GAP
}

const shift = (v: number, size: number, room: number) => Math.max(MARGIN, Math.min(v, room - MARGIN - size))

/** Floats beside `anchor` on `side`, flipped to the other side and shifted to stay inside the window. */
export function Popover({
  anchor,
  side,
  ref: outer,
  ...props
}: {
  anchor: Anchor
  side: 'left' | 'top'
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
        side === 'left' ? [flip(a.left, a.right, w, vw), a.top] : [a.left, flip(a.top, a.bottom, h, vh)]
      el.style.left = `${shift(x, w, vw)}px`
      el.style.top = `${shift(y, h, vh)}px`
    }
    place()
    const observer = new ResizeObserver(place)
    observer.observe(el)
    window.addEventListener('resize', place)
    return () => {
      observer.disconnect()
      window.removeEventListener('resize', place)
    }
  }, [side, ref])
  return <div ref={ref} {...props} className={`popover ${props.className ?? ''}`} />
}
