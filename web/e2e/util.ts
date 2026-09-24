import { expect, type Page } from '@playwright/test'
import { PNG } from 'pngjs'
import { fitView } from '../src/renderer'

export const MM = 72 / 25.4
const A5 = { width: 148 * MM, height: 210 * MM, bleed: 3 * MM }

export async function open(page: Page) {
  await page.setViewportSize({ width: 1400, height: 1100 })
  await page.goto('')
  await expect(page.getByLabel('Zoom')).not.toHaveText('0%')
}

/** Screen position of a point on the sample page, in mm. */
export async function screen(page: Page, x: number, y: number) {
  const canvas = (await page.getByLabel('Page canvas').boundingBox())!
  const view = fitView(A5, canvas.width, canvas.height)
  return [canvas.x + view.x + x * MM * view.zoom, canvas.y + view.y + y * MM * view.zoom] as const
}

export async function drag(page: Page, from: readonly [number, number], to: readonly [number, number]) {
  await page.mouse.move(...from)
  await page.mouse.down()
  await page.mouse.move(...to, { steps: 4 })
  await page.mouse.up()
}

/** RGB pixels of the screen area [x, y, w, h] once the canvas has drawn. */
export async function pixels(page: Page, x: number, y: number, w: number, h: number) {
  await page.waitForTimeout(100)
  const png = PNG.sync.read(await page.screenshot({ clip: { x, y, width: w, height: h } }))
  const out: [number, number, number][] = []
  for (let i = 0; i < png.data.length; i += 4) out.push([png.data[i], png.data[i + 1], png.data[i + 2]])
  return out
}
