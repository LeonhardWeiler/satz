const ZONE = 24
const RATE = 16
const MAX = 1500

/** The speed in px/s for `at` px against the edges `lo` and `hi`, starting `ZONE` px inside them. */
const speed = (at: number, lo: number, hi: number) => {
  const d = at < lo + ZONE ? at - lo - ZONE : at > hi - ZONE ? at - hi + ZONE : 0
  return Math.sign(d) * Math.min(MAX, Math.abs(d) * RATE)
}

/**
 * Scrolls while a drag holds the pointer near or past the edges of `box`, faster the farther out, by whole px
 * passed to `scroll`. `move` gives it the pointer, `stop` ends it.
 */
export function edgeScroll(box: () => DOMRect, scroll: (dx: number, dy: number) => void) {
  let at = { x: 0, y: 0 }
  let frame = 0
  let then = 0
  let left = { x: 0, y: 0 }
  const step = (now: number) => {
    const r = box()
    const [vx, vy] = [speed(at.x, r.left, r.right), speed(at.y, r.top, r.bottom)]
    if (!vx && !vy) return stop()
    const dt = Math.min(now - then, 50) / 1000
    then = now
    left = { x: left.x + vx * dt, y: left.y + vy * dt }
    const [dx, dy] = [Math.trunc(left.x), Math.trunc(left.y)]
    left = { x: left.x - dx, y: left.y - dy }
    frame = requestAnimationFrame(step)
    if (dx || dy) scroll(dx, dy)
  }
  const stop = () => {
    cancelAnimationFrame(frame)
    frame = 0
    left = { x: 0, y: 0 }
  }
  const move = (p: { clientX: number; clientY: number }) => {
    at = { x: p.clientX, y: p.clientY }
    if (frame) return
    then = performance.now()
    frame = requestAnimationFrame(step)
  }
  return { move, stop }
}
