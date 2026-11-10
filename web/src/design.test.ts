import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'

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
  const scale = [0, 2, 4, 8, 12, 16, 24, 32, 48]
  expect(bad((r) => /^(padding|margin|gap|row-gap|column-gap)/.test(r.prop) && px(r.value).some((n) => !scale.includes(n)))).toEqual([])
})

it('uses the radius scale', () => {
  expect(bad((r) => r.prop === 'border-radius' && px(r.value).some((n) => ![0, 2, 4, 6, 8, 12, 999].includes(n)))).toEqual([])
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
