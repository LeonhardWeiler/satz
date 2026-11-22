export type Mat = [number, number, number, number, number, number]
export type Stop = { at: number; color: Float32Array }
export type Paint =
  | { type: 'solid'; color: Float32Array }
  | { type: 'linear' | 'radial'; transform: Mat; stops: Stop[] }
export type Shadow = { offset: [number, number]; blur: number; color: Float32Array }

export type Op = (
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
  | { op: 'image'; image: number; transform: Mat }
  | { op: 'pushClip'; path: Float32Array; invert: boolean }
  | { op: 'popClip' }
  | { op: 'strokePath'; paint: Paint; width: number; cap: number; join: number; path: Float32Array }
  | { op: 'pushLayer'; hash: number; opacity: number; blend: number; blur: number; shadows: Shadow[] }
  | { op: 'pushTransform'; transform: Mat }
  | { op: 'popLayer' | 'beginMask' | 'endMask' | 'popMask' | 'popTransform' }
) & { end?: number }

const SIMPLE = new Map<number, 'endItem' | 'popClip' | 'popLayer' | 'beginMask' | 'endMask' | 'popMask' | 'popTransform'>([
  [2, 'endItem'],
  [7, 'popClip'],
  [10, 'popLayer'],
  [11, 'beginMask'],
  [12, 'endMask'],
  [13, 'popMask'],
  [15, 'popTransform'],
])

export function decode(words: Uint32Array): Op[] {
  const { buffer, byteOffset } = words
  const floats = new Float32Array(buffer, byteOffset, words.length)
  const ops: Op[] = []
  let i = 0
  const next = <T>(list: ArrayLike<T>) => {
    const v = list[i]
    if (v === undefined) throw new Error(`display list: ends at word ${i}`)
    i++
    return v
  }
  const word = () => next(words)
  const float = () => next(floats)
  const f32 = (n: number) => {
    i += n
    if (i > words.length) throw new Error(`display list: ends at word ${words.length}`)
    return new Float32Array(buffer, byteOffset + (i - n) * 4, n)
  }
  const mat = (): Mat => [float(), float(), float(), float(), float(), float()]
  const paint = (): Paint => {
    const type = word()
    if (type === 0) return { type: 'solid', color: f32(4) }
    const transform = mat()
    const stops = Array.from({ length: word() }, () => ({ at: float(), color: f32(4) }))
    return { type: type === 1 ? 'linear' : 'radial', transform, stops }
  }
  const path = () => f32(word())
  while (i < words.length) {
    const code = word()
    switch (code) {
      case 0:
        ops.push({ op: 'page', width: float(), height: float(), bleed: float() })
        break
      case 1:
        ops.push({ op: 'beginItem', item: word(), hash: word() })
        break
      case 3:
        ops.push({ op: 'fillPath', paint: paint(), path: path() })
        break
      case 4: {
        const font = word()
        const size = float()
        const p = paint()
        const n = word()
        const glyphs = new Uint16Array(buffer, byteOffset + i * 4, n)
        i += Math.ceil(n / 2)
        ops.push({ op: 'glyphRun', font, size, paint: p, glyphs, positions: f32(2 * n) })
        break
      }
      case 5:
        ops.push({ op: 'image', image: word(), transform: mat() })
        break
      case 14:
        ops.push({ op: 'pushTransform', transform: mat() })
        break
      case 6:
        ops.push({ op: 'pushClip', invert: word() === 1, path: path() })
        break
      case 8:
        ops.push({ op: 'strokePath', paint: paint(), width: float(), cap: word(), join: word(), path: path() })
        break
      case 9:
        ops.push({
          op: 'pushLayer',
          hash: word(),
          opacity: float(),
          blend: word(),
          blur: float(),
          shadows: Array.from({ length: word() }, () => ({ offset: [float(), float()], blur: float(), color: f32(4) })),
        })
        break
      default: {
        const op = SIMPLE.get(code)
        if (!op) throw new Error(`display list: bad op ${code} at word ${i - 1}`)
        ops.push({ op })
      }
    }
  }
  return link(ops)
}

const OPEN = new Set(['pushClip', 'pushLayer', 'pushTransform', 'beginMask', 'beginItem', 'endMask'])
const CLOSE = new Set(['popClip', 'popLayer', 'popMask', 'popTransform', 'endItem', 'endMask'])

export function link(ops: Op[]) {
  const open: Op[] = []
  ops.forEach((op, i) => {
    if (CLOSE.has(op.op)) {
      const o = open.pop()
      if (!o) throw new Error(`display list: ${op.op} at ${i} closes nothing`)
      o.end = i
    }
    if (OPEN.has(op.op)) open.push(op)
  })
  for (const op of open) op.end = ops.length
  return ops
}

export function close(ops: Op[], at: number) {
  const end = ops[at]?.end
  if (end === undefined) throw new Error(`display list: op ${at} is not open`)
  return end
}
