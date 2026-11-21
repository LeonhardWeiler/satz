import { readdirSync, readFileSync } from 'node:fs'
import { expect, it } from 'vite-plus/test'
import { SELECTION } from './types'

const css = readFileSync(new URL('index.css', import.meta.url), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '')
const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].flatMap(([, sel, body]) =>
  body
    .split(';')
    .map((d) => d.trim())
    .filter((d) => d.includes(':'))
    .map((d) => {
      const i = d.indexOf(':')
      return { sel: sel.trim(), prop: d.slice(0, i).trim(), value: d.slice(i + 1).trim() }
    }),
)

const px = (v: string) => [...v.matchAll(/(-?[\d.]+)px/g)].map((m) => Math.abs(Number(m[1])))

const bad = (test: (r: (typeof rules)[number]) => boolean) =>
  rules.filter((r) => !r.sel.startsWith('@font-face') && test(r)).map((r) => `${r.sel} { ${r.prop}: ${r.value} }`)

const content = /glyph|specimen|text-input/

it('uses the type scale and 2 weights', () => {
  expect(bad((r) => r.prop === 'font-size' && !content.test(r.sel) && px(r.value).some((n) => ![11, 12, 13, 16, 24].includes(n)))).toEqual([])
  expect(bad((r) => r.prop === 'font-weight' && !['400', '600', 'inherit'].includes(r.value))).toEqual([])
  expect(bad((r) => r.prop === 'font' && !content.test(r.sel) && ![11, 12, 13, 16, 24].includes(px(r.value)[0] ?? 12))).toEqual([])
  expect(bad((r) => r.prop === 'font' && /\b(100|200|300|500|700|800|900)\b/.test(r.value))).toEqual([])
})

it('uses the space scale', () => {
  const scale = [0, 2, 4, 8, 12, 16, 24, 32]
  expect(bad((r) => /^(padding|margin|gap|row-gap|column-gap)/.test(r.prop) && px(r.value).some((n) => !scale.includes(n)))).toEqual([])
})

it('uses the radius scale', () => {
  expect(bad((r) => r.prop === 'border-radius' && px(r.value).some((n) => ![0, 4, 6, 12, 999].includes(n)))).toEqual([])
})

it('lifts floating surfaces with one of 2 shadows', () => {
  expect(bad((r) => r.prop === 'box-shadow' && !r.sel.startsWith(':root') && /\d+px \d+px/.test(r.value.replace(/inset[^,]*/g, '')))).toEqual([])
})

it('times motion with the 2 durations', () => {
  const slow = /pf-bar|^\.field\.bad$|fade/
  expect(bad((r) => /^(transition|animation)$/.test(r.prop) && /\d(m?s)\b/.test(r.value) && !slow.test(r.sel))).toEqual([])
})

it('keeps raw colours in the theme', () => {
  const allowed = /^:root|picker|plate|thumb|format-box|ov-ghost canvas|swap|^text$/
  expect(bad((r) => /#[\da-f]{3,8}\b|rgba?\(/i.test(r.value) && !/^mask/.test(r.prop) && !allowed.test(r.sel))).toEqual([])
})

it('paints the loading screen, manifest and icon in the theme colours', () => {
  const token = (sel: string, prop: string) => rules.find((r) => r.sel === sel && r.prop === prop)!.value
  const dark = [token(':root', '--bg'), token(':root', '--fg')]
  const light = [token(":root[data-theme='light']", '--bg'), token(":root[data-theme='light']", '--fg')]
  const html = readFileSync(new URL('../index.html', import.meta.url), 'utf8')
  const manifest = readFileSync(new URL('../public/manifest.webmanifest', import.meta.url), 'utf8')
  const icon = readFileSync(new URL('../public/favicon.svg', import.meta.url), 'utf8').toLowerCase()
  for (const c of [...dark, ...light]) expect(html).toContain(c)
  expect(manifest).toContain(dark[0])
  for (const c of dark) expect(icon).toContain(c)
})

it('selects text on the canvas in the select colour', () => {
  const hex = rules.find((r) => r.sel === ':root' && r.prop === '--select')!.value
  expect(hex).toBe('#' + SELECTION.slice(0, 3).map((c) => Math.round(c * 255).toString(16).padStart(2, '0')).join(''))
})

it('uses every class the stylesheet names', () => {
  const src = [...readdirSync(new URL('.', import.meta.url), { recursive: true, encoding: 'utf8' })]
    .filter((f) => /\.tsx?$/.test(f))
    .map((f) => readFileSync(new URL(f, import.meta.url), 'utf8'))
    .join('')
  const names = new Set(rules.flatMap((r) => [...r.sel.matchAll(/\.([a-z][\w-]*)/g)].map((m) => m[1])))
  expect([...names].filter((c) => !src.includes(c) && !/^(no|plate)-/.test(c))).toEqual([])
})

it('draws lines 1 px and rings 2 px', () => {
  expect(bad((r) => /^border(-(top|right|bottom|left))?(-width)?$/.test(r.prop) && !/picker/.test(r.sel) && px(r.value).some((n) => n !== 1))).toEqual([])
  expect(bad((r) => /^outline(-width)?$/.test(r.prop) && px(r.value).some((n) => n !== 1 && n !== 2))).toEqual([])
})

it('presses controls to 0.93 and honours reduced motion', () => {
  expect(bad((r) => r.sel.includes(':active') && r.prop === 'transform' && r.value !== 'scale(0.93)')).toEqual([])
  expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*\{\s*\*,\s*\*::before,\s*\*::after\s*\{\s*animation: none !important;/)
})

it('keeps greys neutral and muted text at 4.5:1 on every surface', () => {
  for (const theme of [':root', ":root[data-theme='light']"]) {
    const tokens = new Map(rules.filter((r) => r.sel === ':root' || r.sel === theme).map((r) => [r.prop, r.value]))
    const rgba = (v: string): number[] => {
      const ref = /^var\((--[\w-]+)\)$/.exec(v)
      if (ref) return rgba(tokens.get(ref[1])!)
      if (v === 'transparent') return [0, 0, 0, 0]
      if (v.startsWith('#')) {
        const h = v.length === 4 ? [...v.slice(1)].map((c) => c + c).join('') : v.slice(1)
        return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)).concat(1)
      }
      const m = /^color-mix\(in srgb, (.+) (\d+)%, (.+)\)$/.exec(v)!
      const [a, b, t] = [rgba(m[1]), rgba(m[3]), Number(m[2]) / 100]
      const alpha = a[3] * t + b[3] * (1 - t)
      return [0, 1, 2].map((i) => (a[i] * a[3] * t + b[i] * b[3] * (1 - t)) / alpha).concat(alpha)
    }
    const over = (top: number[], under: number[]) => [0, 1, 2].map((i) => top[i] * top[3] + under[i] * (1 - top[3]))
    const lum = (c: number[]) =>
      c.map((v) => v / 255).map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)).reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0)
    const ratio = (a: number[], b: number[]) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05)
    for (const t of ['--bg', '--fg', '--canvas']) expect(new Set(rgba(tokens.get(t)!).slice(0, 3)).size, t).toBe(1)
    const muted = rgba(tokens.get('--muted')!)
    for (const base of ['--bg', '--raised'].map((t) => over(rgba(tokens.get(t)!), [0, 0, 0])))
      for (const s of ['--input', '--hover', '--selected', null])
        expect(ratio(over(muted, base), s ? over(rgba(tokens.get(s)!), base) : base), `${theme} ${s}`).toBeGreaterThanOrEqual(4.5)
  }
})
