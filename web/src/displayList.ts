export type Op =
  | { op: 'page'; width: number; height: number; bleed: number }
  | { op: 'beginItem'; item: number }
  | { op: 'endItem' }
  | { op: 'fillPath'; color: Float32Array; path: Float32Array }
  | {
      op: 'glyphRun'
      font: number
      size: number
      color: Float32Array
      glyphs: Uint16Array
      positions: Float32Array
    }
  | { op: 'image'; image: number; rect: Float32Array }

export function decode(words: Uint32Array): Op[] {
  const { buffer, byteOffset } = words
  const f32 = (at: number, n: number) => new Float32Array(buffer, byteOffset + at * 4, n)
  const ops: Op[] = []
  let i = 0
  while (i < words.length) {
    switch (words[i++]) {
      case 0: {
        const [width, height, bleed] = f32(i, 3)
        ops.push({ op: 'page', width, height, bleed })
        i += 3
        break
      }
      case 1:
        ops.push({ op: 'beginItem', item: words[i++] })
        break
      case 2:
        ops.push({ op: 'endItem' })
        break
      case 3: {
        const color = f32(i, 4)
        const n = words[i + 4]
        ops.push({ op: 'fillPath', color, path: f32(i + 5, n) })
        i += 5 + n
        break
      }
      case 4: {
        const font = words[i]
        const size = f32(i + 1, 1)[0]
        const color = f32(i + 2, 4)
        const n = words[i + 6]
        const glyphs = new Uint16Array(buffer, byteOffset + (i + 7) * 4, n)
        i += 7 + Math.ceil(n / 2)
        ops.push({ op: 'glyphRun', font, size, color, glyphs, positions: f32(i, 2 * n) })
        i += 2 * n
        break
      }
      case 5:
        ops.push({ op: 'image', image: words[i], rect: f32(i + 1, 4) })
        i += 5
        break
      default:
        throw new Error(`display list: bad op ${words[i - 1]} at word ${i - 1}`)
    }
  }
  return ops
}
