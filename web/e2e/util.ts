import { expect, type Page } from '@playwright/test'
import { PNG } from 'pngjs'
import { fitView, type Sheet } from '../src/renderer'

export const MM = 72 / 25.4

/** Opens the app; a spread fits at the zoom of a single page in a window 2000 px wide. */
export async function open(page: Page, width = 1400) {
  await page.setViewportSize({ width, height: 1100 })
  await page.goto('')
  await expect(page.getByLabel('Zoom')).not.toHaveText('0%')
}

/**
 * Screen position of a point in mm on page `i` of the fitted spread the canvas shows,
 * by default the right or only one.
 */
export async function screen(page: Page, x: number, y: number, i?: number) {
  const canvas = page.getByLabel('Page canvas')
  const box = (await canvas.boundingBox())!
  const sheets: Sheet[] = JSON.parse((await canvas.getAttribute('data-sheets'))!)
  const view = fitView(sheets, box.width, box.height)
  const sx = sheets[i ?? sheets.length - 1].x + x * MM
  return [box.x + view.x + sx * view.zoom, box.y + view.y + y * MM * view.zoom] as const
}

export async function drag(page: Page, from: readonly [number, number], to: readonly [number, number]) {
  await page.mouse.move(...from)
  await page.mouse.down()
  await page.mouse.move(...to, { steps: 4 })
  await page.mouse.up()
}

/** Waits until the canvas has drawn the frame it has asked for, if any. */
export async function drawn(page: Page) {
  await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))))
}

/** RGB pixels of the screen area [x, y, w, h] once the canvas has drawn. */
export async function pixels(page: Page, x: number, y: number, w: number, h: number) {
  await drawn(page)
  const png = PNG.sync.read(await page.screenshot({ clip: { x, y, width: w, height: h } }))
  const out: [number, number, number][] = []
  for (let i = 0; i < png.data.length; i += 4) out.push([png.data[i], png.data[i + 1], png.data[i + 2]])
  return out
}

/** RGB pixels at screen points, from one screenshot once the canvas has drawn. */
export async function colors(page: Page, points: (readonly [number, number])[]) {
  await drawn(page)
  const png = PNG.sync.read(await page.screenshot())
  return points.map(([x, y]) => {
    const i = (Math.round(y) * png.width + Math.round(x)) * 4
    return [png.data[i], png.data[i + 1], png.data[i + 2]]
  })
}

export const STORY =
  'Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut labore ' +
  'et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum.'

/** Screen position of the out-port of a text frame from (x0, y0) to (x1, y1) in mm, or its in-port, on page `i` of the spread. */
export async function port(page: Page, [x0, y0, x1, y1]: number[], out: boolean, i?: number) {
  const [l, t] = await screen(page, x0, y0, i)
  const [r, b] = await screen(page, x1, y1, i)
  return (out ? [r, b - 16] : [l, t + 16]) as [number, number]
}

/** Adds a page and draws a fixed text frame from (x0, y0) to (x1, y1) in mm on it. */
export async function frameOnNewPage(page: Page, box: number[]) {
  await page.getByRole('navigation', { name: 'Pages' }).getByRole('button', { name: 'Add page' }).click()
  await page.keyboard.press('t')
  await drag(page, await screen(page, box[0], box[1]), await screen(page, box[2], box[3]))
}
