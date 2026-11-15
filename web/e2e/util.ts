import { expect, test as base, type Locator, type Page } from '@playwright/test'
import { join } from 'node:path'
import { PNG } from 'pngjs'
import { fitView, type Sheet } from '../src/renderer'

export const MM = 72 / 25.4

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

export async function open(page: Page, width = 1400) {
  page.on('filechooser', () => {})
  await page.setViewportSize({ width: width + 48, height: 1160 })
  await page.goto('')
  await page.getByRole('dialog', { name: 'Projects' }).getByRole('button', { name: 'Sample' }).click()
  await page.mouse.move(0, 0)
  await expect(page.getByLabel('Zoom')).not.toHaveText('0%')
}

export async function choose(select: Locator, option: string) {
  await select.click()
  await select.page().getByRole('menu', { name: (await select.getAttribute('aria-label'))! }).getByRole('menuitemradio', { name: option, exact: true }).click()
}

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

export async function autosaved(page: Page, dirty = true) {
  const stored = () =>
    page.evaluate(
      (dirty) =>
        new Promise<boolean>((done) => {
          const r = indexedDB.open('satz')
          r.onsuccess = () => {
            const get = r.result.transaction('files').objectStore('files').get(location.hash.slice(1))
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

export function png(name: string, width: number, height: number, rgb: (x: number, y: number) => number[]) {
  const out = new PNG({ width, height })
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) out.data.set([...rgb(x, y), 255], (y * width + x) * 4)
  return { name, mimeType: 'image/png', buffer: PNG.sync.write(out) }
}

export async function place(page: Page, file: { name: string; mimeType: string; buffer: Buffer }, at = [74, 105]) {
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('button', { name: 'Place image' }).click()
  await (await chooser).setFiles(file)
  const [x, y] = await screen(page, at[0], at[1])
  await page.mouse.click(x, y)
}

export async function drawn(page: Page) {
  const frames = () => page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))))
  await frames()
  if (await page.locator('canvas[data-decoding]').count()) {
    await page.waitForFunction(() => !document.querySelector('canvas[data-decoding]'))
    await frames()
  }
}

export async function pixels(page: Page, x: number, y: number, w: number, h: number) {
  await drawn(page)
  const png = PNG.sync.read(await page.screenshot({ clip: { x, y, width: w, height: h } }))
  const out: [number, number, number][] = []
  for (let i = 0; i < png.data.length; i += 4) out.push([png.data[i], png.data[i + 1], png.data[i + 2]])
  return out
}

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

export async function port(page: Page, [x0, y0, x1, y1]: number[], out: boolean, i?: number) {
  const [l, t] = await screen(page, x0, y0, i)
  const [r, b] = await screen(page, x1, y1, i)
  const inset = Math.min(24, (b - t) / 4)
  return (out ? [r, b - inset] : [l, t + inset]) as [number, number]
}

export async function frameOnNewPage(page: Page, box: number[]) {
  await addPage(page)
  await page.keyboard.press('t')
  await drag(page, await screen(page, box[0], box[1]), await screen(page, box[2], box[3]))
}

export const near = ([r, g, b]: number[], [R, G, B]: number[]) => Math.max(Math.abs(r - R), Math.abs(g - G), Math.abs(b - B)) <= 24

export async function overview(page: Page) {
  const o = page.getByRole('region', { name: 'Page overview' })
  if (!(await o.count())) await page.getByRole('button', { name: 'Page overview' }).click()
  return o
}

export const option = (page: Page, n: number) =>
  page.getByRole('region', { name: 'Page overview' }).getByRole('option', { name: `Page ${n}`, exact: true })

export const current = (page: Page, n: number) =>
  page.getByRole('navigation', { name: 'Spreads' }).getByRole('button', { name: `Page ${n}`, exact: true })


export async function showPage(page: Page, n: number) {
  await (await overview(page)).getByRole('option', { name: `Page ${n}`, exact: true }).click()
  await expect(page.getByRole('region', { name: 'Page overview' })).toHaveCount(0)
}

export async function addMaster(page: Page) {
  await (await overview(page)).getByRole('button', { name: 'New master' }).click()
  await expect(page.getByRole('region', { name: 'Page overview' })).toHaveCount(0)
}

export async function pageCount(page: Page, n: number) {
  await expect((await overview(page)).getByRole('listbox', { name: 'Pages' }).getByRole('option')).toHaveCount(n)
  await page.keyboard.press('.')
}

export async function exportButton(page: Page) {
  return (await preflight(page)).locator('.pf-go')
}

export async function preflight(page: Page) {
  const region = page.getByRole('region', { name: 'Preflight' })
  if (!(await region.count())) {
    await page.keyboard.press('Control+Alt+Y')
    await page.keyboard.press('Shift+1')
  }
  return region
}

export async function openExample(page: Page, name: string) {
  await page.addInitScript(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
  await open(page, 2000)
  const chooser = page.waitForEvent('filechooser')
  await page.keyboard.press('Control+o')
  await (await chooser).setFiles(join(import.meta.dirname, '../../examples', name))
  await expect(page).toHaveTitle(`${name} - Satz`)
  await page.mouse.move(1, 1)
}

export async function addVariable(page: Page, kind: 'Color' | 'Number' | 'Font' | 'Boolean') {
  const panel = await assets(page)
  await panel.getByRole('button', { name: 'Add variable' }).click()
  await page.getByRole('menu', { name: 'Create variable' }).getByRole('menuitem', { name: kind }).click()
  const edit = page.getByRole('dialog', { name: /^Edit / })
  await expect(edit).toBeVisible()
  return edit
}

export async function addPage(page: Page) {
  const add = page.getByRole('navigation', { name: 'Spreads' }).getByRole('button', { name: 'Add page' })
  await ((await add.count()) ? add.click() : page.keyboard.press('Control+Alt+KeyP'))
}

export async function assets(page: Page) {
  await page.getByRole('tab', { name: 'Assets' }).click()
  return page.getByRole('tabpanel', { name: 'Assets' })
}
