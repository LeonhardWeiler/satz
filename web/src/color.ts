import { neutral as engineNeutral, preview, resolve as engineResolve, toCmyk } from './engine/engine'
import type { Color, ColorMode, Fill, Process, Scope } from './model'
export type { Color, ColorMode, Process } from './types'

export type Hsv = [h: number, s: number, v: number]

export const resolve = (c: Color, scope: Scope): Process =>
  typeof c === 'object' && !('cmyk' in c) ? engineResolve(c, scope) : c

export function rgb(c: Color, scope: Scope) {
  const p = resolve(c, scope)
  if (typeof p === 'number') return p >>> 8
  const [cyan = 0, magenta = 0, yellow = 0, black = 0] = p.cmyk
  return preview(cyan, magenta, yellow, black)
}

export const alpha = (c: Color) => (typeof c === 'number' ? ((c & 0xff) / 255) * 100 : c.alpha * 100)

export function withAlpha(c: Color, percent: number): Color {
  const a = Math.min(100, Math.max(0, percent)) / 100
  return typeof c === 'number' ? ((c & ~0xff) | Math.round(a * 255)) >>> 0 : { ...c, alpha: a }
}

export function css(c: Color, scope: Scope) {
  const p = resolve(c, scope)
  return `#${rgb(p, scope).toString(16).padStart(6, '0')}${Math.round(alpha(p) * 2.55).toString(16).padStart(2, '0')}`
}

export function colorText(c: Color, scope: Scope) {
  const p = resolve(c, scope)
  return typeof p === 'number' ? `#${(p >>> 8).toString(16).padStart(6, '0').toUpperCase()}` : p.cmyk.map((v) => Math.round(v * 100)).join(' ')
}

export function fromRgb(rgb: number, like: Color, mode: ColorMode): Color {
  if (mode === 'rgb') return withAlpha(((rgb << 8) | 0xff) >>> 0, alpha(like))
  return { cmyk: [...toCmyk(rgb)].map((v) => Math.round(v * 100) / 100), alpha: alpha(like) / 100 }
}

export function toHsv(rgb: number): Hsv {
  const byte = (s: number) => ((rgb >>> s) & 0xff) / 255
  const [r, g, b] = [byte(16), byte(8), byte(0)]
  const v = Math.max(r, g, b)
  const d = v - Math.min(r, g, b)
  const h = d === 0 ? 0 : v === r ? ((g - b) / d + 6) % 6 : v === g ? (b - r) / d + 2 : (r - g) / d + 4
  return [h * 60, v === 0 ? 0 : d / v, v]
}

export function fromHsv([h, s, v]: Hsv) {
  const f = (n: number) => {
    const k = (n + h / 60) % 6
    return Math.round((v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255)
  }
  return (f(5) << 16) | (f(3) << 8) | f(1)
}

export const solid = (color: Color): Fill => ({ type: 'solid', color, stops: [], transform: [1, 0, 0, 1, 0, 0], visible: true })

export const neutral = (name: 'black' | 'white' | 'gray', mode: ColorMode) => engineNeutral(name, mode)

export const cmykText = (cmyk: number[]) => cmyk.map((v, i) => `${'CMYK'[i]}:${Math.round(v * 1000) / 10}`).join(' ')

export function parseCmyk(text: string): number[] | null {
  const parts = text.trim().replace(/:\s*/g, '').split(/\s*[\s,/]\s*/)
  const values = parts.map((p, i) => {
    const m = /^([cmyk]?)(\d+(?:\.\d+)?)%?$/i.exec(p)
    return m && (!m[1] || m[1].toUpperCase() === 'CMYK'[i]) && Number(m[2]) <= 100 ? Number(m[2]) / 100 : NaN
  })
  return values.length === 4 && values.every((v) => v >= 0) ? values : null
}

export function parseHex(text: string) {
  const t = text.trim().replace(/^#/, '')
  const full = t.length === 3 ? [...t].map((c) => c + c).join('') : t
  return /^[0-9a-f]{6}$/i.test(full) ? parseInt(full, 16) : null
}

export function parseColor(text: string, like: Color, mode: ColorMode): Color | null {
  const cmyk = mode === 'cmyk' ? parseCmyk(text) : null
  if (cmyk) return { cmyk, alpha: alpha(like) / 100 }
  const hex = parseHex(text)
  return hex === null ? null : fromRgb(hex, like, mode)
}
