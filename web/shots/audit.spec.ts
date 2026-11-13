import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs'
import type { Page } from '@playwright/test'
import { example, OUT, props, rest, section, select, start, test } from './lib'

const FILE = `${OUT}/audit.txt`
const BOX = '.field, .select, .segmented, .tabs, .button, .primary, .icon-button, .swatch, input:not([type=checkbox]), [role=slider], canvas, .check, .bound-name, [role=option], [role=menuitem], [role=menuitemradio], [role=menuitemcheckbox], textarea, [role=treeitem] > div, .settings-card, .pal-it, .format, .layer, .iss, h2 + .icon-button'

type Item = { kind: 'box' | 'text'; key: string; sec: string; x: number; y: number; r: number; b: number; fs?: number; fw?: string; pad?: number }

async function dump(page: Page, root: string) {
  return page.evaluate(
    ([root, BOX]) => {
      const out: Item[] = []
      const vis = (el: Element) => {
        const r = el.getBoundingClientRect()
        for (let e: Element | null = el; e; e = e.parentElement) {
          const s = getComputedStyle(e)
          if (s.visibility === 'hidden' || s.opacity === '0' || s.display === 'none') return false
        }
        return r.width > 0 && r.height > 0
      }
      const secOf = (el: Element) => {
        const s = el.closest('[role=region], section, [role=dialog], dialog, [role=menu], [role=listbox], nav, header, [role=tabpanel], aside')
        if (!s) return '?'
        return (s.getAttribute('aria-label') ?? s.querySelector('h2,h3')?.textContent ?? s.tagName.toLowerCase() + '.' + s.className).slice(0, 30)
      }
      const name = (el: Element) => {
        const t = el.getAttribute('aria-label') ?? el.getAttribute('title') ?? el.textContent?.trim() ?? ''
        return `${el.tagName.toLowerCase()}.${[...el.classList].join('.')}[${t.slice(0, 24)}]`
      }
      const top = document.querySelector(root)
      if (!top) return out
      const boxes = [...top.querySelectorAll(BOX)].filter((el) => vis(el) && !el.parentElement!.closest(BOX))
      for (const el of boxes) {
        const r = el.getBoundingClientRect()
        let pad: number | undefined
        const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT)
        for (let n = w.nextNode(); n; n = w.nextNode()) {
          if (!n.textContent!.trim() || !vis(n.parentElement!)) continue
          const rg = document.createRange()
          rg.selectNodeContents(n)
          const t = rg.getBoundingClientRect()
          if (t.width) pad = Math.round((t.left - r.left) * 2) / 2
          break
        }
        out.push({ kind: 'box', key: name(el), sec: secOf(el), x: r.left, y: r.top, r: r.right, b: r.bottom, pad })
      }
      const w = document.createTreeWalker(top, NodeFilter.SHOW_TEXT)
      for (let n = w.nextNode(); n; n = w.nextNode()) {
        const p = n.parentElement!
        if (!n.textContent!.trim() || !vis(p) || p.closest(BOX)) continue
        const rg = document.createRange()
        rg.selectNodeContents(n)
        const t = rg.getBoundingClientRect()
        if (!t.width) continue
        const s = getComputedStyle(p)
        out.push({ kind: 'text', key: `${p.tagName.toLowerCase()}.${[...p.classList].join('.')}"${n.textContent!.trim().slice(0, 24)}"`, sec: secOf(p), x: t.left, y: t.top, r: t.right, b: t.bottom, fs: parseFloat(s.fontSize), fw: s.fontWeight })
      }
      return out
    },
    [root, BOX] as const,
  )
}


async function inks(page: Page, root: string) {
  return page.evaluate((root) => {
    const ctx = document.createElement('canvas').getContext('2d', { willReadFrequently: true })!
    const rgba = (c: string) => {
      ctx.clearRect(0, 0, 1, 1)
      ctx.fillStyle = c
      ctx.fillRect(0, 0, 1, 1)
      const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data
      return [r, g, b, a / 255]
    }
    const over = (t: number[], u: number[]) => [0, 1, 2].map((i) => t[i] * t[3] + u[i] * (1 - t[3]))
    const lum = (c: number[]) => {
      const [r, g, b] = c.map((v) => (v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)
      return 0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    const out = new Set<string>()
    const top = document.querySelector(root)
    if (!top) return []
    const w = document.createTreeWalker(top, NodeFilter.SHOW_TEXT)
    for (let n = w.nextNode(); n; n = w.nextNode()) {
      const p = n.parentElement!
      if (!n.textContent!.trim() || !p.checkVisibility({ opacityProperty: true, visibilityProperty: true })) continue
      let dim = false
      const layers: number[][] = []
      for (let e: Element | null = p; e; e = e.parentElement) {
        const s = getComputedStyle(e)
        if (+s.opacity < 1) dim = true
        const c = rgba(s.backgroundColor)
        if (c[3] > 0) layers.push(c)
        if (c[3] === 1) break
      }
      if (dim) continue
      let bg = [255, 255, 255]
      for (const l of layers.reverse()) bg = over(l, bg)
      const s = getComputedStyle(p)
      const fg = over(rgba(s.color), bg)
      const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x)
      const cr = (a + 0.05) / (b + 0.05)
      const big = parseFloat(s.fontSize) >= 18.66 || (parseFloat(s.fontSize) >= 14 && +s.fontWeight >= 700)
      if (cr < (big ? 3 : 4.5)) out.add(`${cr.toFixed(2)} ${p.tagName.toLowerCase()}.${[...p.classList].join('.')} "${n.textContent!.trim().slice(0, 20)}" ${s.color}`)
    }
    return [...out]
  }, root)
}

const f = (n: number) => Math.round(n * 2) / 2

function analyse(state: string, items: Item[]) {
  const lines: string[] = [`\n=== ${state} (${items.length})`]
  if (process.env.FULL) for (const i of [...items].sort((a, b) => a.y - b.y || a.x - b.x)) lines.push(`    ${i.kind} ${i.sec.slice(0, 14).padEnd(14)} x${f(i.x)} y${f(i.y)} r${f(i.r)} h${f(i.b - i.y)}${i.fs ? ' fs' + i.fs + '/' + i.fw : ''}${i.pad !== undefined ? ' pad' + i.pad : ''} ${i.key.slice(0, 60)}`)
  const near = (vals: [number, string][], what: string) => {
    vals.sort((a, b) => a[0] - b[0])
    for (let i = 1; i < vals.length; i++) {
      const d = vals[i][0] - vals[i - 1][0]
      if (d > 0.6 && d <= 6) lines.push(`  ${what} ${f(vals[i - 1][0])} vs ${f(vals[i][0])}: ${vals[i - 1][1]}  |  ${vals[i][1]}`)
    }
  }
  near(items.map((i) => [i.x, `${i.sec} ${i.key}`]), 'LEFT')
  near(items.filter((i) => i.kind === 'box').map((i) => [i.r, `${i.sec} ${i.key}`]), 'RIGHT')
  const hs = new Map<string, Set<number>>()
  for (const i of items.filter((i) => i.kind === 'box')) {
    const k = i.key.replace(/\[.*$/, '')
    hs.set(k, (hs.get(k) ?? new Set()).add(f(i.b - i.y)))
  }
  lines.push('  HEIGHTS ' + [...hs].map(([k, v]) => `${k}=${[...v].join('/')}`).join('  '))
  const pads = new Map<string, Set<number>>()
  for (const i of items.filter((i) => i.kind === 'box' && i.pad !== undefined)) {
    const k = i.key.replace(/\[.*$/, '')
    pads.set(k, (pads.get(k) ?? new Set()).add(i.pad!))
  }
  lines.push('  TEXT INSET ' + [...pads].map(([k, v]) => `${k}=${[...v].join('/')}`).join('  '))
  const fonts = new Set(items.filter((i) => i.kind === 'text').map((i) => `${i.fs}/${i.fw}`))
  lines.push('  FONTS ' + [...fonts].join(' '))
  const bySec = new Map<string, Item[]>()
  for (const i of items) bySec.set(i.sec, [...(bySec.get(i.sec) ?? []), i])
  for (const [sec, its] of bySec) {
    const boxes = its.filter((i) => i.kind === 'box').sort((a, b) => a.y - b.y)
    const rows: Item[][] = []
    for (const b of boxes) {
      const row = rows.find((r) => r.some((o) => b.y < o.b - 2 && b.b > o.y + 2))
      if (row) row.push(b)
      else rows.push([b])
    }
    for (const row of rows) {
      const tops = new Set(row.map((i) => f(i.y)))
      const hts = new Set(row.map((i) => f(i.b - i.y)))
      if (row.length > 1 && (tops.size > 1 || hts.size > 1)) lines.push(`  ROW ${sec}: ` + row.map((i) => `${i.key}@${f(i.y)}h${f(i.b - i.y)}`).join(' , '))
    }
    const gaps = rows.slice(1).map((r, k) => f(Math.min(...r.map((i) => i.y)) - Math.max(...rows[k].map((i) => i.b))))
    if (gaps.length) lines.push(`  GAPS ${sec}: ${gaps.join(' ')}`)
  }
  appendFileSync(FILE, lines.join('\n') + '\n')
}

const run = async (page: Page, state: string, root: string) => {
  analyse(state, await dump(page, root))
  const low = await inks(page, root)
  if (low.length) appendFileSync(FILE, low.map((l) => '  CONTRAST ' + l).join('\n') + '\n')
}

test.beforeAll(() => { mkdirSync(OUT, { recursive: true }); writeFileSync(FILE, '') })

test('audit', async ({ page }) => {
  test.setTimeout(180_000)
  await start(page, { h: 1400 })
  await page.keyboard.press('Escape')
  await rest(page)
  await run(page, 'shell', '.top, .toolbar, header')
  await run(page, 'right: page', '.right')
  await run(page, 'left: layers', '.left')
  await select(page, 'Star')
  await run(page, 'right: star', '.right')
  await select(page, 'Rectangle')
  await run(page, 'right: rectangle', '.right')
  await select(page, 'Satz sets type…')
  await run(page, 'right: text', '.right')
  await props(page).getByRole('button', { name: 'Type options' }).click()
  const to = page.getByRole('dialog', { name: 'Type options' })
  await run(page, 'type options: character', '[role=dialog][aria-label="Type options"]')
  await to.getByRole('tab', { name: 'Paragraph' }).click()
  await run(page, 'type options: paragraph', '[role=dialog][aria-label="Type options"]')
  await to.getByRole('tab', { name: 'Tabs' }).click()
  await to.getByRole('button', { name: 'Add tab stop' }).click()
  await run(page, 'type options: tabs', '[role=dialog][aria-label="Type options"]')
  await page.keyboard.press('Escape')
  await select(page, 'Star')
  await section(page, 'Fill').getByRole('button', { name: 'Fill color' }).first().click()
  await run(page, 'color picker', '[role=dialog][aria-label="Fill color"]')
  await page.keyboard.press('Escape')
  await page.getByRole('region', { name: 'Swatches' }).getByRole('button', { name: 'Add swatch' }).click()
  await run(page, 'edit swatch', '[role=dialog]:last-of-type')
  await run(page, 'swatches', '[aria-label=Swatches]')
  await page.keyboard.press('Escape')
  await page.getByRole('tab', { name: 'Assets' }).click()
  const vars = page.locator('.left').getByRole('region', { name: 'Variables', exact: true })
  await vars.getByRole('button', { name: 'Add variable' }).click()
  await page.getByRole('menu', { name: 'Create variable' }).getByRole('menuitem').first().click()
  await rest(page)
  await run(page, 'left: assets', '.left')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Control+Alt+y')
  await rest(page)
  await run(page, 'right: preflight', '.right')
  await page.keyboard.press('Control+Alt+y')
  await page.keyboard.press('Control+,')
  await run(page, 'settings', 'dialog.settings')
  await page.getByRole('dialog', { name: 'Settings' }).getByRole('button', { name: 'Screen size' }).click()
  await run(page, 'settings: screen', 'dialog.settings')
  await page.getByRole('dialog', { name: 'Settings' }).getByRole('button', { name: 'Keyboard shortcuts' }).click()
  await run(page, 'settings: shortcuts', 'dialog.settings')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Control+k')
  await run(page, 'palette', 'dialog.palette')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Control+f')
  await run(page, 'find', '[aria-label="Find and replace"]')
  await page.keyboard.press('Escape')
  await page.mouse.click(800, 700, { button: 'right' })
  await run(page, 'context menu', '[role=menu]')
  await page.keyboard.press('Escape')
  await page.keyboard.press('.')
  await run(page, 'page overview', '.overview')
})

test('dialogs', async ({ page }) => {
  await start(page, { doc: null })
  await run(page, 'projects', '[role=dialog][aria-label=Projects], dialog')
  await page.getByRole('dialog', { name: 'Projects' }).getByRole('button', { name: 'New document' }).click()
  await run(page, 'new document', 'dialog')
})

test('booklet', async ({ page }) => {
  await example(page, 'booklet.satz', { h: 1400 })
  await page.keyboard.press('Escape')
  await run(page, 'booklet right: page', '.right')
  await run(page, 'booklet left', '.left')
})

test('popovers', async ({ page }) => {
  await start(page, { h: 1400 })
  await page.keyboard.press('Escape')
  await select(page, 'Star')
  await section(page, 'Effects').getByRole('button', { name: 'Add effect' }).click()
  await rest(page)
  await run(page, 'right: star with effect', '.right')
  await section(page, 'Effects').getByRole('button', { name: 'Effect settings' }).click()
  await run(page, 'effect options', '[role=dialog]')
  await page.keyboard.press('Escape')
  await select(page, 'Star')
  await section(page, 'Stroke').getByRole('button', { name: 'Stroke options' }).click()
  await section(page, 'Layer').getByRole('button', { name: 'Layer options' }).click()
  await run(page, 'star options open', '.right')
  await page.keyboard.press('Escape')
  await page.getByRole('tab', { name: 'Assets' }).click()
  const vars = page.locator('.left').getByRole('region', { name: 'Variables', exact: true })
  await vars.getByRole('button', { name: 'Add variable' }).click()
  await page.getByRole('menu', { name: 'Create variable' }).getByRole('menuitem', { name: 'Color' }).click()
  const edit = page.getByRole('dialog', { name: /^Edit / })
  await edit.getByRole('button', { name: /Add mode/ }).click()
  await run(page, 'variable editor', '[role=dialog]')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Control+Alt+z')
  await run(page, 'history', '[role=dialog][aria-label="Version history"], dialog[aria-label="Version history"]')
})
