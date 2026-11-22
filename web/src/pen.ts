export type Anchor = { x: number; y: number; hx: number; hy: number }

export function penPath(anchors: Anchor[], closed: boolean): number[] {
  const [first, ...rest] = anchors
  if (!first) return []
  const path = [0, first.x, first.y]
  const segment = (a: Anchor, b: Anchor) => {
    if (!a.hx && !a.hy && !b.hx && !b.hy) path.push(1, b.x, b.y)
    else path.push(4, a.x + a.hx, a.y + a.hy, b.x - b.hx, b.y - b.hy, b.x, b.y)
  }
  let last = first
  for (const a of rest) {
    segment(last, a)
    last = a
  }
  if (closed) {
    segment(last, first)
    path.push(5)
  }
  return path
}
