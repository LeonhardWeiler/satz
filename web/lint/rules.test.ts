import { describe, it } from 'vite-plus/test'
import { RuleTester } from 'vite-plus/lint/plugins-dev'
import plugin from './index.ts'

RuleTester.describe = describe
RuleTester.it = it
const tester = new RuleTester({ languageOptions: { parserOptions: { lang: 'tsx' } } })
const run = (name: string, valid: string[], invalid: string[]) => {
  const rule = plugin.rules[name]
  if (!rule) throw new Error(`no rule ${name}`)
  tester.run(name, rule, { valid, invalid: invalid.map((code) => ({ code, errors: 1 })) })
}

run(
  'no-test-timing',
  ['await expect(page.getByRole("tooltip")).toBeVisible()', 'await expect.poll(() => n()).toBe(1)', 'const o = { timeout: 1 }'],
  [
    'await page.waitForTimeout(150)',
    'await new Promise((done) => setTimeout(done, 100))',
    'await new Promise((done) => window.setTimeout(done, 100))',
    'await expect(page.getByText("x")).toBeVisible({ timeout: 30000 })',
    'await expect.poll(() => n(), { timeout: 15000 }).toBe(1)',
    'test.setTimeout(600_000)',
    'test.slow()',
  ],
)

run(
  'no-hex-color',
  ['const c = getComputedStyle(el).getPropertyValue("--bad")', 'location.hash = "#add-page"', 'const s = `#${id}`'],
  ['const OVERSET = "#ff6b5e"', 'const c = "#fff"', 'const svg = `<path stroke="#000"/>`', '<path fill="#1d1d1b" />'],
)

run(
  'no-cached-rejection',
  [
    'p ??= load().catch((e) => { p = undefined; throw e })',
    'const names = once(() => fetch(url).then((r) => r.text()))',
    'gpu ??= ck.MakeWebGLContext(off)',
    'handle ??= await pickers.showSaveFilePicker(o)',
  ],
  ['db ??= new Promise((done) => done(1))', 'names ??= fetch(url).then((r) => r.text()).catch(() => new Map())', 'names ??= fetch(url)', 'p ??= load().then(parse)'],
)

run(
  'no-unit-text',
  ['const s = length(pt)', '<p>Type a unit, e.g. 12pt or 1cm</p>', 'const s = `${n} pages in ${name}`', 'const s = "Nudge by one step"'],
  ['const s = "Nudge by 0.1 mm, with Shift by 1 mm"', "const g = ['5 mm grid', 1]", '<p>Bleed of 3 mm</p>', 'const s = `Bleed of 3 pt, ${x}`'],
)

run(
  'way-out',
  [
    'class Editor { loadImage(bytes) { this.set({ placing: [{ url: URL.createObjectURL(new Blob([bytes])) }] }) } set(patch) { for (const p of this.placing) URL.revokeObjectURL(p.url) } }',
    'function save(bytes) { const url = URL.createObjectURL(new Blob([bytes])); a.href = url; a.click(); URL.revokeObjectURL(url) }',
  ],
  ['img.src = URL.createObjectURL(blob)'],
)
