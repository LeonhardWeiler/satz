import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { expect, test, type Page } from '@playwright/test'
import { PNG } from 'pngjs'
import { fitView } from '../src/renderer'

const WIDTH = 1400
const HEIGHT = 1100
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

async function open(page: Page) {
  await page.setViewportSize({ width: WIDTH, height: HEIGHT })
  await page.goto('')
  await expect(page.getByLabel('Zoom')).not.toHaveText('0%')
}

async function expectCanvasMatchesPdf(page: Page) {
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const dir = mkdtempSync(join(tmpdir(), 'satz-'))
  const pdf = join(dir, 'satz.pdf')
  const download = page.waitForEvent('download')
  await page.getByRole('button', { name: 'Export PDF' }).click()
  await (await download).saveAs(pdf)

  const boxes = execFileSync('mutool', ['pages', pdf]).toString()
  const bleedBox = pageBox(boxes, 'BleedBox')
  const trimBox = pageBox(boxes, 'TrimBox')
  const bleed = trimBox.l - bleedBox.l
  const width = trimBox.r - trimBox.l
  const height = trimBox.t - trimBox.b
  const view = fitView({ width, height, bleed }, canvas.width, canvas.height)

  const x = Math.round(canvas.x + view.x - bleed * view.zoom)
  const y = Math.round(canvas.y + view.y - bleed * view.zoom)
  const w = Math.round((width + 2 * bleed) * view.zoom)
  const h = Math.round((height + 2 * bleed) * view.zoom)
  await page.waitForTimeout(100)
  const shot = PNG.sync.read(await page.screenshot({ clip: { x, y, width: w, height: h } }))

  const raster = join(dir, 'satz.png')
  execFileSync('mutool', ['draw', '-q', '-b', 'BleedBox', '-r', `${view.zoom * 72}`, '-o', raster, pdf])
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
}

test('canvas matches the exported pdf', async ({ page }) => {
  await open(page)
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
