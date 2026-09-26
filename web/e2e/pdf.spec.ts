import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { expect, test, type Page } from '@playwright/test'
import { PNG } from 'pngjs'
import { fitView, type Sheet } from '../src/renderer'
import { STORY, drag, drawn, frameOnNewPage, open, place, png, port, screen } from './util'

const EDGE = 4
const BLOCK = 4
const BACKGROUND = 0x1e
const MAX_DIFF = 48
const MAX_SHARE = 0.0005

function pageBox(xml: string, name: string) {
  const m = xml.match(new RegExp(`<${name} l="([\\d.]+)" b="([\\d.]+)" r="([\\d.]+)" t="([\\d.]+)"`))!
  const [l, b, r, t] = m.slice(1).map(Number)
  return { l, b, r, t }
}

/** Trim size and bleed of page `n` of `pdf`. */
function sheet(pdf: string, n: number) {
  const boxes = execFileSync('mutool', ['pages', pdf, String(n)]).toString()
  const bleedBox = pageBox(boxes, 'BleedBox')
  const trimBox = pageBox(boxes, 'TrimBox')
  return { x: 0, width: trimBox.r - trimBox.l, height: trimBox.t - trimBox.b, bleed: trimBox.l - bleedBox.l }
}

/** Compares the canvas with page `n` of the exported pdf, which the canvas shows in the spread of the pages `spread`. */
async function expectCanvasMatchesPdf(page: Page, n = 1, spread = [n]) {
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const dir = mkdtempSync(join(tmpdir(), 'satz-'))
  const pdf = join(dir, 'satz.pdf')
  const download = page.waitForEvent('download')
  await page.getByRole('button', { name: 'Export PDF' }).click()
  await (await download).saveAs(pdf)

  const sheets: Sheet[] = spread.map((k) => sheet(pdf, k))
  if (sheets.length === 2) sheets[0].x = -sheets[0].width
  const { x: left, width, height, bleed } = sheets[spread.indexOf(n)]
  const view = fitView(sheets, canvas.width, canvas.height)

  const x = Math.round(canvas.x + view.x + (left - bleed) * view.zoom)
  const y = Math.round(canvas.y + view.y - bleed * view.zoom)
  const w = Math.round((width + 2 * bleed) * view.zoom)
  const h = Math.round((height + 2 * bleed) * view.zoom)
  await drawn(page)
  // The export may leave a message, such as the preflight warning, over the canvas.
  const shot = PNG.sync.read(await page.screenshot({ clip: { x, y, width: w, height: h }, style: '.status { display: none }' }))

  const raster = join(dir, 'satz.png')
  execFileSync('mutool', ['draw', '-q', '-O', '0', '-b', 'BleedBox', '-r', `${view.zoom * 72}`, '-o', raster, pdf, String(n)])
  const ref = PNG.sync.read(readFileSync(raster))
  expect(Math.abs(ref.width - w)).toBeLessThanOrEqual(1)
  expect(Math.abs(ref.height - h)).toBeLessThanOrEqual(1)
  const cw = Math.min(w, ref.width)
  const ch = Math.min(h, ref.height)

  const edge = (v: number, a: number, b: number) => Math.abs(v - a) <= EDGE || Math.abs(v - b) <= EDGE
  const t = bleed * view.zoom
  const sample = (img: PNG, px: number, py: number, c: number, pasteboard: boolean) => {
    let sum = 0
    for (let dy = 0; dy < BLOCK; dy++) {
      for (let dx = 0; dx < BLOCK; dx++) {
        const i = ((py + dy) * img.width + px + dx) * 4 + c
        sum += pasteboard && img.data[i] === BACKGROUND ? 255 : img.data[i]
      }
    }
    return sum / (BLOCK * BLOCK)
  }
  let compared = 0
  let differing = 0
  for (let py = 0; py + BLOCK <= ch; py += BLOCK) {
    for (let px = 0; px + BLOCK <= cw; px += BLOCK) {
      const cx = px + BLOCK / 2
      const cy = py + BLOCK / 2
      if (edge(cx, 0, w - 1) || edge(cy, 0, h - 1) || edge(cx, t, w - 1 - t) || edge(cy, t, h - 1 - t)) continue
      const inBleed = cx < t || cy < t || cx > w - 1 - t || cy > h - 1 - t
      const d = Math.max(
        ...[0, 1, 2].map((c) => Math.abs(sample(shot, px, py, c, inBleed) - sample(ref, px, py, c, false))),
      )
      compared++
      if (d > MAX_DIFF) differing++
    }
  }
  expect(differing / compared).toBeLessThan(MAX_SHARE)
  return pdf
}

test('canvas matches the exported pdf, whose text is the text of every line', async ({ page }) => {
  await open(page)
  const pdf = await expectCanvasMatchesPdf(page)
  const text = execFileSync('mutool', ['draw', '-q', '-F', 'text', '-o', '-', pdf]).toString()
  expect(text.replace(/-\n/g, '').replace(/\s+/g, ' ')).toContain(
    'Satz sets type in the browser. The engine shapes this paragraph with harfrust, breaks it into lines with the ' +
      'Knuth-Plass algorithm and justifies every line but the last to the width of its frame. The canvas and the PDF ' +
      'draw the same glyphs from the same font.',
  )
})

test('a placed image exports as the canvas shows it', async ({ page }) => {
  await open(page)
  await page.keyboard.press('Control+a')
  await page.keyboard.press('Delete')
  // MuPDF stretches images to whole pixels, so the ramp has no sharp edges and fades into the page's white.
  const fade = (x: number, y: number) => Math.min(1, x / 60, (1199 - x) / 60, y / 60, (599 - y) / 60)
  const ramp = (x: number, y: number) => [x / 5, y / 3, 255 - x / 5].map((c) => 255 + (c - 255) * fade(x, y))
  await place(page, png('ramp.png', 1200, 600, ramp))
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem', { name: 'ramp.png' })).toBeVisible()
  await page.keyboard.press('Escape')
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  await page.mouse.move(canvas.x + 2, canvas.y + 2)
  await expectCanvasMatchesPdf(page)
})

test('draw, move, undo and redo a rectangle, then export it', async ({ page }) => {
  await open(page)
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  const x = page.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')
  await expect(rects).toHaveCount(2)

  await page.keyboard.press('r')
  const cx = canvas.x + canvas.width / 2
  const cy = canvas.y + canvas.height / 2
  await page.mouse.move(cx - 60, cy - 40)
  await page.mouse.down()
  await page.mouse.move(cx + 20, cy + 40, { steps: 4 })
  await page.mouse.up()
  await expect(rects).toHaveCount(3)
  const drawn = await x.inputValue()

  await page.mouse.move(cx - 20, cy)
  await page.mouse.down()
  await page.mouse.move(cx + 40, cy, { steps: 4 })
  await page.mouse.up()
  await expect(x).not.toHaveValue(drawn)

  await page.keyboard.press('Control+z')
  await expect(x).toHaveValue(drawn)
  await page.keyboard.press('Control+z')
  await expect(rects).toHaveCount(2)
  await page.keyboard.press('Control+Shift+z')
  await expect(rects).toHaveCount(3)

  await page.keyboard.press('Escape')
  await page.mouse.move(canvas.x + 2, canvas.y + 2)
  await expectCanvasMatchesPdf(page)
})

test('marquee, group, enter the group and undo', async ({ page }) => {
  await open(page)
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const at = (fx: number, fy: number) => [canvas.x + canvas.width * fx, canvas.y + canvas.height * fy] as const
  const title = page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })
  const groups = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Group', exact: true })

  await page.mouse.move(...at(0.2, 0.45))
  await page.mouse.down()
  await page.mouse.move(...at(0.6, 0.8), { steps: 5 })
  await page.mouse.up()
  await expect(title).toHaveText('2 layers')
  await page.keyboard.press('Control+g')
  await expect(title).toHaveText('Group')
  await expect(groups).toHaveCount(1)
  await page.mouse.dblclick(...at(0.5, 0.5))
  await expect(title).toHaveText(/^Satz sets type/)
  await page.keyboard.press('Escape')
  await expect(title).toHaveText('Group')
  await page.keyboard.press('Control+z')
  await expect(groups).toHaveCount(0)
})

test('copy, paste and alt-drag duplicate', async ({ page }) => {
  await open(page)
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  await rects.first().click()
  await page.keyboard.press('Control+c')
  await page.keyboard.press('Control+v')
  await expect(rects).toHaveCount(3)

  const [x, y] = [canvas.x + canvas.width * 0.5, canvas.y + canvas.height * 0.1]
  await page.keyboard.press('Escape')
  await page.mouse.move(x, y)
  await page.keyboard.down('Alt')
  await page.mouse.down()
  await page.mouse.move(x + 40, y + 40, { steps: 4 })
  await page.mouse.up()
  await page.keyboard.up('Alt')
  await expect(rects).toHaveCount(4)
  await page.keyboard.press('Control+z')
  await expect(rects).toHaveCount(3)
})

test('draw shapes and a closed pen path, then export them', async ({ page }) => {
  await open(page)
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const at = (fx: number, fy: number) => [canvas.x + canvas.width * fx, canvas.y + canvas.height * fy] as const
  const layers = page.getByRole('tree', { name: 'Layers' })
  const names = ['Ellipse', 'Arrow', 'Star', 'Vector']
  const count = (name: string) => layers.getByRole('button', { name, exact: true })
  const before = await Promise.all(names.map((n) => count(n).count()))
  const drag = async (from: readonly [number, number], to: readonly [number, number]) => {
    await page.mouse.move(...from)
    await page.mouse.down()
    await page.mouse.move(...to, { steps: 4 })
    await page.mouse.up()
  }

  await page.keyboard.press('o')
  await drag(at(0.35, 0.45), at(0.45, 0.55))
  await page.keyboard.press('Shift+L')
  await drag(at(0.5, 0.45), at(0.6, 0.55))
  await page.getByRole('button', { name: 'Shape tools' }).click()
  await page.getByRole('menuitemradio', { name: /Star/ }).click()
  await drag(at(0.35, 0.6), at(0.45, 0.7))

  await page.keyboard.press('p')
  await page.mouse.click(...at(0.5, 0.6))
  await drag(at(0.65, 0.62), at(0.68, 0.7))
  await page.mouse.click(...at(0.55, 0.72))
  await page.mouse.click(...at(0.5, 0.6))

  for (const [i, name] of names.entries()) await expect(count(name)).toHaveCount(before[i] + 1)
  await page.keyboard.press('Escape')
  await page.mouse.move(canvas.x + 2, canvas.y + 2)
  await expectCanvasMatchesPdf(page)
})

test('a cmyk document exports cmyk and spot colours and matches the canvas', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  await panel.getByRole('combobox', { name: 'Color mode' }).selectOption('CMYK')

  await page.getByRole('region', { name: 'Swatches' }).getByRole('button', { name: 'Add swatch' }).click()
  const editor = page.getByRole('dialog', { name: 'Edit swatch' })
  await editor.getByRole('textbox', { name: 'Name' }).fill('HKS 43')
  await editor.getByRole('checkbox', { name: 'Spot color' }).check()
  const ink = async (dialog: typeof editor, values: number[]) => {
    for (const [i, name] of ['Cyan', 'Magenta', 'Yellow', 'Black'].entries()) {
      await dialog.getByRole('textbox', { name }).fill(String(values[i]))
      await dialog.getByRole('textbox', { name }).press('Enter')
    }
  }
  await ink(editor, [100, 60, 0, 0])
  await page.keyboard.press('Escape')

  await rects.last().click()
  await panel.getByRole('button', { name: 'Fill color' }).click()
  await ink(page.getByRole('dialog', { name: 'Fill color' }), [0, 100, 100, 0])
  await page.keyboard.press('Escape')
  await rects.first().click()
  await panel.getByRole('button', { name: 'Fill color' }).click()
  const picker = page.getByRole('dialog', { name: 'Fill color' })
  await picker.getByRole('tab', { name: 'Swatches' }).click()
  await picker.getByRole('option', { name: 'HKS 43' }).click()
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await expect(panel.getByRole('heading', { level: 2 })).toHaveText('Page')
  await page.mouse.move(1, 1)

  const pdf = await expectCanvasMatchesPdf(page)
  const trace = execFileSync('mutool', ['draw', '-F', 'trace', '-o', '-', pdf]).toString()
  expect(trace).toContain('colorspace="DeviceCMYK" color="0 1 1 0"')
  expect(trace).toContain('colorspace="Separation(DeviceCMYK,HKS 43)" color="1"')
})

test('an auto layout frame with a colour variable in a second mode matches the canvas', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  await panel.getByRole('button', { name: 'Local variables' }).click()
  const dialog = page.getByRole('dialog', { name: 'Local variables' })
  await dialog.getByRole('button', { name: 'Create collection' }).click()
  await dialog.getByRole('button', { name: 'Create variable' }).click()
  await page.getByRole('menuitem', { name: 'Color' }).click()
  await dialog.getByRole('button', { name: 'Add mode' }).click()
  await dialog.getByRole('button', { name: 'Color 1 in Mode 2' }).click()
  const hex = page.getByRole('dialog', { name: 'Color 1 in Mode 2' }).getByRole('textbox', { name: 'Hex' })
  await hex.fill('2c9f5a')
  await hex.press('Enter')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')

  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await page.keyboard.press('Shift+A')
  await panel.getByRole('combobox', { name: 'Collection 1 mode' }).selectOption('Mode 2')
  await panel.getByRole('textbox', { name: 'Top padding in mm' }).fill('4')
  await panel.getByRole('textbox', { name: 'Top padding in mm' }).press('Enter')
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await page.keyboard.press('Control+d')
  await panel.getByRole('button', { name: 'Fill color' }).click()
  const picker = page.getByRole('dialog', { name: 'Fill color' })
  await picker.getByRole('tab', { name: 'Swatches' }).click()
  await picker.getByRole('option', { name: 'Color 1' }).click()
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await expect(panel.getByRole('heading', { level: 2 })).toHaveText('Page')
  await page.mouse.move(1, 1)
  await expectCanvasMatchesPdf(page)
})

test('formatted text matches the canvas', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  for (const [name, value] of [['W in mm', '110'], ['Font size in pt', '13'], ['Line height in pt', '21'], ['Letter spacing in %', '4'], ['Paragraph spacing in pt', '8']]) {
    await panel.getByRole('textbox', { name }).fill(value)
    await panel.getByRole('textbox', { name }).press('Enter')
  }
  await panel.getByRole('radio', { name: 'Align right' }).click()
  await panel.getByRole('combobox', { name: 'Hyphenation language' }).selectOption('German')
  for (const [name, value] of [['Top inset in mm', '3'], ['Columns', '2'], ['Gutter in mm', '4'], ['Baseline grid in pt', '21']]) {
    await panel.getByRole('textbox', { name }).fill(value)
    await panel.getByRole('textbox', { name }).press('Enter')
  }
  await panel.getByRole('radio', { name: 'Align middle' }).click()
  await page.keyboard.press('Escape')
  await page.mouse.move(1, 1)
  const pdf = await expectCanvasMatchesPdf(page)
  const trace = execFileSync('mutool', ['draw', '-F', 'trace', '-o', '-', pdf]).toString()
  expect(trace.split('glyph="hyphen"').length - 1).toBeGreaterThan(1)
})

test('every page is exported and the second one matches the canvas', async ({ page }) => {
  await open(page)
  await page.getByRole('navigation', { name: 'Pages' }).getByRole('button', { name: 'Add page' }).click()
  const panel = page.getByRole('complementary', { name: 'Properties' })
  for (const [name, value] of [['W in mm', '120'], ['H in mm', '160'], ['Bleed in mm', '5']]) {
    await panel.getByRole('textbox', { name }).fill(value)
    await panel.getByRole('textbox', { name }).press('Enter')
  }
  await page.keyboard.press('Shift+1')
  await page.keyboard.press('o')
  await drag(page, await screen(page, 30, 40), await screen(page, 110, 90))
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.mouse.move(1, 1)
  const pdf = await expectCanvasMatchesPdf(page, 2)
  expect(execFileSync('mutool', ['pages', pdf]).toString().match(/<page /g)).toHaveLength(2)
})

test('a master drawn under a page matches the canvas', async ({ page }) => {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const at = (fx: number, fy: number) => [canvas.x + canvas.width * fx, canvas.y + canvas.height * fy] as const
  await page.keyboard.press('o')
  await page.mouse.move(...at(0.2, 0.7))
  await page.mouse.down()
  await page.mouse.move(...at(0.8, 0.95), { steps: 4 })
  await page.mouse.up()
  await pages.getByRole('button', { name: 'Page 1', exact: true }).click()
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' }).selectOption('A-Master')
  await page.mouse.move(1, 1)
  await expectCanvasMatchesPdf(page)
})

test('text threaded across two pages matches the canvas and reads as one story', async ({ page }) => {
  await open(page, 2000)
  const a = [20, 20, 70, 50]
  await frameOnNewPage(page, a)
  await page.keyboard.type(STORY)
  await page.keyboard.press('Escape')
  await page.mouse.click(...(await port(page, a, true)))
  await page.getByRole('navigation', { name: 'Pages' }).getByRole('button', { name: 'Add page' }).click()
  await page.mouse.click(...(await screen(page, 30, 100)))
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.mouse.move(1, 1)
  const pdf = await expectCanvasMatchesPdf(page, 3, [2, 3])
  const text = execFileSync('mutool', ['draw', '-q', '-F', 'text', '-o', '-', pdf, '2-3']).toString()
  expect(text.replace(/\s+/g, ' ')).toContain(STORY)
})

test('a layer across the spine prints on both pages of its spread and matches the canvas', async ({ page }) => {
  await open(page, 2000)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  await pages.getByRole('button', { name: 'Add page' }).click()
  await pages.getByRole('button', { name: 'Add page' }).click()
  await page.keyboard.press('o')
  await drag(page, await screen(page, 110, 60, 0), await screen(page, 40, 110))
  await pages.getByRole('button', { name: 'Page 2', exact: true }).click()
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Ellipse', exact: true })).toHaveCount(1)
  await page.keyboard.press('Escape')
  await page.mouse.move(1, 1)
  const pdf = await expectCanvasMatchesPdf(page, 3, [2, 3])
  const trace = execFileSync('mutool', ['draw', '-F', 'trace', '-o', '-', pdf, '2-3']).toString()
  expect(trace.match(/<fill_path/g)).toHaveLength(2)
})

test('the pages of a spread show their sides of a master spread and match the canvas', async ({ page }) => {
  await open(page, 2000)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  await page.keyboard.press('o')
  await drag(page, await screen(page, 110, 150, 0), await screen(page, 40, 200))
  await page.keyboard.press('r')
  await drag(page, await screen(page, 10, 10, 0), await screen(page, 40, 25, 0))
  for (let i = 0; i < 2; i++) {
    await pages.getByRole('button', { name: 'Add page' }).click()
    await panel.getByRole('combobox', { name: 'Master' }).selectOption('A-Master')
  }
  await page.mouse.move(1, 1)
  await expectCanvasMatchesPdf(page, 2, [2, 3])
})
