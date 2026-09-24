import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { expect, test } from '@playwright/test'
import { PNG } from 'pngjs'
import { fitView } from '../src/renderer'

const WIDTH = 900
const HEIGHT = 1100
const EDGE = 4
const BLOCK = 4
const BACKGROUND = 0x1e

function pageBox(xml: string, name: string) {
  const m = xml.match(new RegExp(`<${name} l="([\\d.]+)" b="([\\d.]+)" r="([\\d.]+)" t="([\\d.]+)"`))!
  const [l, b, r, t] = m.slice(1).map(Number)
  return { l, b, r, t }
}

test('canvas matches the exported pdf', async ({ page }) => {
  await page.setViewportSize({ width: WIDTH, height: HEIGHT })
  await page.goto('')
  await expect(page.getByLabel('Zoom')).not.toHaveText('0%')

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
  const view = fitView({ width, height, bleed }, WIDTH, HEIGHT)

  const x = Math.round(view.x - bleed * view.zoom)
  const y = Math.round(view.y - bleed * view.zoom)
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
      if (d > 64) differing++
    }
  }
  expect(differing / compared).toBeLessThan(0.001)
})
