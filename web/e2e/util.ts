import { expect, test as base, type Page } from '@playwright/test'
import { join } from 'node:path'
import { PNG } from 'pngjs'
import { fitView, type Sheet } from '../src/renderer'

export const MM = 72 / 25.4

/** Fails a test that logs an error in the browser console, unless it expects errors with `test.info().annotations.push({ type: 'errors' })`. */
export const test = base.extend<{ console: void }>({
  console: [
    async ({ page }, use, info) => {
      const errors: string[] = []
      page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
      page.on('pageerror', (e) => errors.push(e.message))
      await use()
      if (!info.annotations.some((a) => a.type === 'errors')) expect(errors).toEqual([])
    },
    { auto: true },
  ],
})
export { expect }

/** Opens the app with a canvas of `width` − 496 × 1100 px; a spread fits at the zoom of a single page at 2000. */
export async function open(page: Page, width = 1400) {
  // Playwright turns on catching file choosers without waiting; a chooser opened right after may still get past it.
  page.on('filechooser', () => {})
  await page.setViewportSize({ width: width + 48, height: 1160 })
  await page.goto('')
  await page.getByRole('dialog', { name: 'New document' }).getByRole('button', { name: 'Sample' }).click()
  await page.mouse.move(0, 0)
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

/** Waits until the autosave holds a document, by default one with unsaved changes. */
export async function autosaved(page: Page, dirty = true) {
  const stored = () =>
    page.evaluate(
      (dirty) =>
        new Promise<boolean>((done) => {
          const r = indexedDB.open('satz')
          r.onsuccess = () => {
            const get = r.result.transaction('files').objectStore('files').get('doc')
            get.onsuccess = () => {
              done(!!get.result && (!dirty || get.result.dirty))
              r.result.close()
            }
          }
        }),
      dirty,
    )
  await expect.poll(stored).toBe(true)
}

/** A PNG file `name` of `width` × `height` pixels coloured `rgb(x, y)`. */
export function png(name: string, width: number, height: number, rgb: (x: number, y: number) => number[]) {
  const out = new PNG({ width, height })
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) out.data.set([...rgb(x, y), 255], (y * width + x) * 4)
  return { name, mimeType: 'image/png', buffer: PNG.sync.write(out) }
}

/** Places the image `file` with the toolbar's Place image. */
export async function place(page: Page, file: { name: string; mimeType: string; buffer: Buffer }) {
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Place image' }).click()
  await (await chooser).setFiles(file)
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
  await addPage(page)
  await page.keyboard.press('t')
  await drag(page, await screen(page, box[0], box[1]), await screen(page, box[2], box[3]))
}

/** Whether the colour `p` is within 24 of `q` in each channel. */
export const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

/** The page overview, opened by its button in the top bar if it is closed. */
export async function overview(page: Page) {
  const o = page.getByRole('region', { name: 'Page overview' })
  if (!(await o.count())) await page.getByRole('button', { name: 'Page overview' }).click()
  return o
}

/** Page `n` in the page overview, which must be open. */
export const option = (page: Page, n: number) =>
  page.getByRole('region', { name: 'Page overview' }).getByRole('option', { name: `Page ${n}`, exact: true })

/** The button of page `n` in the spread switcher, pressed while the page is current. */
export const current = (page: Page, n: number) =>
  page.getByRole('navigation', { name: 'Spreads' }).getByRole('button', { name: `Page ${n}`, exact: true })

/** Adds a page after the current one in the page overview and closes it. */
export async function addPage(page: Page) {
  const o = await overview(page)
  await o.getByRole('button', { name: 'Add page' }).click()
  await page.keyboard.press('.')
  await expect(o).toHaveCount(0)
}

/** Shows page `n` by a double click in the page overview, which closes it. */
export async function showPage(page: Page, n: number) {
  await (await overview(page)).getByRole('option', { name: `Page ${n}`, exact: true }).dblclick()
  await expect(page.getByRole('region', { name: 'Page overview' })).toHaveCount(0)
}

/** Adds a master in the page overview, which shows it. */
export async function addMaster(page: Page) {
  await (await overview(page)).getByRole('button', { name: 'New master' }).click()
  await expect(page.getByRole('region', { name: 'Page overview' })).toHaveCount(0)
}

/** The number of pages the page overview lists; closes it again. */
export async function pageCount(page: Page, n: number) {
  await expect((await overview(page)).getByRole('option')).toHaveCount(n)
  await page.keyboard.press('.')
}

/** The export button of the preflight, which the Export button in the top bar opens; fits the narrower canvas again. */
export async function exportButton(page: Page) {
  await page.getByRole('button', { name: 'Export', exact: true }).click()
  await page.keyboard.press('Shift+1')
  return page.getByRole('region', { name: 'Preflight' }).locator('.pf-go')
}

/** The preflight, opened with Ctrl Alt Y if it is closed; fits the narrower canvas again. */
export async function preflight(page: Page) {
  const region = page.getByRole('region', { name: 'Preflight' })
  if (!(await region.count())) {
    await page.keyboard.press('Control+Alt+Y')
    await page.keyboard.press('Shift+1')
  }
  return region
}

/** Opens the file `name` from examples/. */
export async function openExample(page: Page, name: string) {
  await page.addInitScript(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
  await open(page, 2000)
  const chooser = page.waitForEvent('filechooser')
  await page.keyboard.press('Control+o')
  await (await chooser).setFiles(join(import.meta.dirname, '../../examples', name))
  await expect(page).toHaveTitle(`${name} — Satz`)
  await page.mouse.move(1, 1)
}
