/** A pen point in pt; (hx, hy) is its outgoing handle, the incoming one is mirrored. */
export type Anchor = { x: number; y: number; hx: number; hy: number }

/** Display-list path commands through the anchors. */
export function penPath(anchors: Anchor[], closed: boolean): number[] {
  if (!anchors.length) return []
  const path = [0, anchors[0].x, anchors[0].y]
  const segment = (a: Anchor, b: Anchor) => {
    if (!a.hx && !a.hy && !b.hx && !b.hy) path.push(1, b.x, b.y)
    else path.push(4, a.x + a.hx, a.y + a.hy, b.x - b.hx, b.y - b.hy, b.x, b.y)
  }
  for (let i = 1; i < anchors.length; i++) segment(anchors[i - 1], anchors[i])
  if (closed) {
    segment(anchors.at(-1)!, anchors[0])
    path.push(5)
  }
  return path
}
