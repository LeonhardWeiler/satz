import { neutral as engineNeutral, preview, resolve as engineResolve, toCmyk } from './engine/engine'
import type { Scope } from './model'

export type ColorMode = 'rgb' | 'cmyk'
/**
 * RGB is 0xRRGGBBAA; CMYK components, tint and alpha are 0..1. A swatch's tint only applies to spot colours;
 * alpha multiplies what a swatch or variable holds.
 */
export type Color =
  | number
  | { cmyk: number[]; alpha: number }
  | { swatch: string; tint: number; alpha: number }
  | { variable: string; alpha: number }
export type Process = number | { cmyk: number[]; alpha: number }
export type Hsv = [h: number, s: number, v: number]

/** The process colour a swatch or variable stands for. */
export const resolve = (c: Color, scope: Scope): Process =>
  typeof c === 'object' && !('cmyk' in c) ? engineResolve(c, scope) : c

/** Screen colour as 0xRRGGBB. */
export function rgb(c: Color, scope: Scope) {
  const p = resolve(c, scope)
  return typeof p === 'number' ? p >>> 8 : preview(p.cmyk[0], p.cmyk[1], p.cmyk[2], p.cmyk[3])
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

/** `rgb` in the document's mode, with the alpha of `like`. */
export function fromRgb(rgb: number, like: Color, mode: ColorMode): Color {
  if (mode === 'rgb') return withAlpha(((rgb << 8) | 0xff) >>> 0, alpha(like))
  return { cmyk: [...toCmyk(rgb)].map((v) => Math.round(v * 100) / 100), alpha: alpha(like) / 100 }
}

export function toHsv(rgb: number): Hsv {
  const [r, g, b] = [16, 8, 0].map((s) => ((rgb >>> s) & 0xff) / 255)
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

export const neutral = (name: 'black' | 'white' | 'gray', mode: ColorMode) => engineNeutral(name, mode) as Color
