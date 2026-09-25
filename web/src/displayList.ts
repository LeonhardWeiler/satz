export type Stop = { at: number; color: Float32Array }
export type Paint =
  | { type: 'solid'; color: Float32Array }
  | { type: 'linear' | 'radial'; transform: Float32Array; stops: Stop[] }
export type Shadow = { offset: Float32Array; blur: number; color: Float32Array }

export type Op =
  | { op: 'page'; width: number; height: number; bleed: number }
  | { op: 'beginItem'; item: number; hash: number }
  | { op: 'endItem' }
  | { op: 'fillPath'; paint: Paint; path: Float32Array }
  | {
      op: 'glyphRun'
      font: number
      size: number
      paint: Paint
      glyphs: Uint16Array
      positions: Float32Array
    }
  | { op: 'image'; image: number; rect: Float32Array }
  | { op: 'pushClip'; path: Float32Array; invert: boolean }
  | { op: 'popClip' }
  | { op: 'strokePath'; paint: Paint; width: number; cap: number; join: number; path: Float32Array }
  | { op: 'pushLayer'; hash: number; opacity: number; blend: number; blur: number; shadows: Shadow[] }
  | { op: 'popLayer' | 'beginMask' | 'endMask' | 'popMask' }

const SIMPLE = { 2: 'endItem', 7: 'popClip', 10: 'popLayer', 11: 'beginMask', 12: 'endMask', 13: 'popMask' } as const

export function decode(words: Uint32Array): Op[] {
  const { buffer, byteOffset } = words
  const f32 = (at: number, n: number) => new Float32Array(buffer, byteOffset + at * 4, n)
  const ops: Op[] = []
  let i = 0
  const paint = (): Paint => {
    const type = words[i++]
    if (type === 0) {
      i += 4
      return { type: 'solid', color: f32(i - 4, 4) }
    }
    const transform = f32(i, 6)
    const n = words[i + 6]
    i += 7
    const stops = Array.from({ length: n }, (_, k) => ({ at: f32(i + 5 * k, 1)[0], color: f32(i + 5 * k + 1, 4) }))
    i += 5 * n
    return { type: type === 1 ? 'linear' : 'radial', transform, stops }
  }
  const path = () => {
    const n = words[i]
    i += 1 + n
    return f32(i - n, n)
  }
  while (i < words.length) {
    const code = words[i++]
    switch (code) {
      case 0: {
        const [width, height, bleed] = f32(i, 3)
        ops.push({ op: 'page', width, height, bleed })
        i += 3
        break
      }
      case 1:
        ops.push({ op: 'beginItem', item: words[i], hash: words[i + 1] })
        i += 2
        break
      case 3: {
        const p = paint()
        ops.push({ op: 'fillPath', paint: p, path: path() })
        break
      }
      case 4: {
        const font = words[i]
        const size = f32(i + 1, 1)[0]
        i += 2
        const p = paint()
        const n = words[i]
        const glyphs = new Uint16Array(buffer, byteOffset + (i + 1) * 4, n)
        i += 1 + Math.ceil(n / 2)
        ops.push({ op: 'glyphRun', font, size, paint: p, glyphs, positions: f32(i, 2 * n) })
        i += 2 * n
        break
      }
      case 5:
        ops.push({ op: 'image', image: words[i], rect: f32(i + 1, 4) })
        i += 5
        break
      case 6: {
        const invert = words[i++] === 1
        ops.push({ op: 'pushClip', invert, path: path() })
        break
      }
      case 8: {
        const p = paint()
        const width = f32(i, 1)[0]
        const [cap, join] = [words[i + 1], words[i + 2]]
        i += 3
        ops.push({ op: 'strokePath', paint: p, width, cap, join, path: path() })
        break
      }
      case 9: {
        const hash = words[i]
        const [opacity] = f32(i + 1, 1)
        const blend = words[i + 2]
        const [blur] = f32(i + 3, 1)
        const n = words[i + 4]
        i += 5
        const shadows = Array.from({ length: n }, (_, k) => ({
          offset: f32(i + 7 * k, 2),
          blur: f32(i + 7 * k + 2, 1)[0],
          color: f32(i + 7 * k + 3, 4),
        }))
        i += 7 * n
        ops.push({ op: 'pushLayer', hash, opacity, blend, blur, shadows })
        break
      }
      default:
        if (!(code in SIMPLE)) throw new Error(`display list: bad op ${code} at word ${i - 1}`)
        ops.push({ op: SIMPLE[code as keyof typeof SIMPLE] })
    }
  }
  return ops
}

const OPEN = new Set(['pushClip', 'pushLayer', 'beginMask', 'beginItem'])
const CLOSE = new Set(['popClip', 'popLayer', 'popMask', 'endItem', 'endMask'])

/** Index of the op that closes the one opened at `at`; `endMask` closes `beginMask` and opens up to `popMask`. */
export function close(ops: Op[], at: number) {
  let depth = 1
  for (let i = at + 1; i < ops.length; i++) {
    const op = ops[i].op
    if (CLOSE.has(op) && --depth === 0) return i
    if (OPEN.has(op) || op === 'endMask') depth++
  }
  return ops.length
}
