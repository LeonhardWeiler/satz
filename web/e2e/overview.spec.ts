import type { Locator } from '@playwright/test'
import { expect, test, addMaster, addPage, current, open, option, overview } from './util'

test('dot opens the page overview on the current page with drawn thumbnails, and dot or escape closes it', async ({ page }) => {
  await open(page)
  const region = page.getByRole('region', { name: 'Page overview' })
  await page.keyboard.press('.')
  await expect(option(page, 1)).toHaveAttribute('aria-selected', 'true')
  const colours = () =>
    option(page, 1)
      .locator('canvas')
      .evaluate((c: HTMLCanvasElement) => {
        const copy = new OffscreenCanvas(c.width, c.height).getContext('2d')!
        copy.drawImage(c, 0, 0)
        const { data } = copy.getImageData(0, 0, c.width, c.height)
        const seen = new Set<number>()
        for (let i = 0; i < data.length; i += 4) seen.add((data[i] << 16) | (data[i + 1] << 8) | data[i + 2])
        return seen.size
      })
  await expect.poll(colours).toBeGreaterThan(3)
  await page.keyboard.press('.')
  await expect(region).toHaveCount(0)
  await page.keyboard.press('.')
  await page.keyboard.press('Escape')
  await expect(region).toHaveCount(0)
})

test('delete removes the selected pages with a toast, and undo restores them', async ({ page }) => {
  await open(page)
  await addPage(page)
  await addPage(page)
  const pages = (await overview(page)).getByRole('option')
  await option(page, 1).click({ modifiers: ['Control'] })
  await option(page, 2).click({ modifiers: ['Control'] })
  await option(page, 3).click({ modifiers: ['Shift'] })
  await page.keyboard.press('Delete')
  await expect(page.getByText('Deleted 2 pages. Ctrl Z restores')).toBeVisible()
  await expect(pages).toHaveCount(1)
  await expect(option(page, 1)).toHaveAttribute('aria-selected', 'true')
  await page.keyboard.press('Delete')
  await expect(page.getByText('A document keeps at least 1 page')).toBeVisible()
  await page.keyboard.press('Control+z')
  await expect(pages).toHaveCount(3)
})

test('new master and master like page open a new master, and escape leaves it', async ({ page }) => {
  await open(page)
  const banner = page.getByText(/Editing master/)
  await addMaster(page)
  await expect(banner).toContainText('A-Master')
  await page.keyboard.press('Escape')
  await expect(banner).toHaveCount(0)
  await expect(current(page, 1)).toHaveAttribute('aria-pressed', 'true')

  await overview(page)
  await option(page, 1).click({ button: 'right' })
  await page.getByRole('menuitem', { name: 'Master like page' }).click()
  await expect(banner).toContainText('B-Master')
})

test('page keys move between spreads and fit them, and shift+2 and ctrl+0 zoom', async ({ page }) => {
  await open(page)
  const zoom = page.getByLabel('Zoom')
  const fit = (await zoom.textContent())!
  for (let i = 0; i < 3; i++) await addPage(page)
  await page.keyboard.press('Home')
  await expect(current(page, 1)).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('PageDown')
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('End')
  await expect(current(page, 4)).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('PageUp')
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')

  await page.keyboard.press('Home')
  await page.keyboard.press('Control+0')
  await expect(zoom).toHaveText('100%')
  await page.keyboard.press('Control+=')
  await expect(zoom).toHaveText('125%')
  await page.keyboard.press('Control+-')
  await page.keyboard.press('Control+-')
  await expect(zoom).toHaveText('80%')
  await page.keyboard.press('Control+0')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Sun' }).click()
  await page.keyboard.press('Shift+2')
  await expect(zoom).not.toHaveText('100%')
  await page.keyboard.press('Shift+1')
  await expect(zoom).toHaveText(fit)
  await page.keyboard.press('Control+0')
  await page.keyboard.press('PageDown')
  await expect(zoom).not.toHaveText('100%')
})

test('the masters column is pulled wider with its thumbnails, the pages zoom and flow in columns', async ({ page }) => {
  await open(page)
  await addMaster(page)
  await addPage(page)
  await addPage(page)
  const region = await overview(page)
  const master = region.getByRole('option', { name: 'A-Master' })
  const width = async (l: typeof master) => Math.round((await l.boundingBox())!.width)
  const before = await width(master)
  const sep = (await region.getByRole('separator', { name: 'Masters width' }).boundingBox())!
  await page.mouse.move(sep.x + 2, sep.y + 200)
  await page.mouse.down()
  await page.mouse.move(sep.x + 102, sep.y + 200, { steps: 4 })
  await page.mouse.up()
  await expect.poll(() => width(master)).toBe(before + 100)
  const first = option(page, 1)
  const small = await width(first)
  await region.getByRole('button', { name: 'Zoom in' }).click()
  await expect.poll(() => width(first)).toBeGreaterThan(small)
  const top = async (n: number) => Math.round((await option(page, n).boundingBox())!.y)
  expect(await top(2)).toBeGreaterThan(await top(1))
  await region.getByRole('textbox', { name: 'Columns' }).hover()
  expect(await region.getByRole('button', { name: 'Decrease Columns' }).evaluate((b) => b.matches(':hover'))).toBe(false)
  await region.getByRole('textbox', { name: 'Columns' }).fill('2')
  await region.getByRole('textbox', { name: 'Columns' }).press('Enter')
  await expect.poll(() => top(2)).toBe(await top(1))
})

test('the pages header stays in view when the overview scrolls', async ({ page }) => {
  await open(page)
  for (let i = 0; i < 3; i++) await addPage(page)
  const region = await overview(page)
  await page.setViewportSize({ width: 1000, height: 600 })
  const zoomIn = region.getByRole('button', { name: 'Zoom in' })
  while (await zoomIn.isEnabled()) await zoomIn.click()
  await region.getByRole('textbox', { name: 'Columns' }).fill('4')
  await region.getByRole('textbox', { name: 'Columns' }).press('Enter')
  const before = await zoomIn.boundingBox()
  const pages = region.locator('.ov-col').last()
  await pages.evaluate((c) => c.scrollTo(c.scrollWidth, c.scrollHeight))
  await expect(pages).not.toHaveJSProperty('scrollTop', 0)
  await expect(pages).not.toHaveJSProperty('scrollLeft', 0)
  expect(await zoomIn.boundingBox()).toEqual(before)
  await expect(zoomIn).toBeInViewport({ ratio: 1 })
  await expect(region.getByRole('button', { name: 'Zoom out' })).toBeInViewport({ ratio: 1 })
})

test('ctrl click and a band select pages or masters, shift click a range, the selection drags to its new place and a click opens', async ({ page }) => {
  await open(page)
  for (let i = 0; i < 4; i++) await addPage(page)
  for (let i = 0; i < 3; i++) {
    await addMaster(page)
    await page.keyboard.press('Escape')
  }
  const region = await overview(page)
  const order = (name: string) => region.getByRole('listbox', { name }).getByRole('option').evaluateAll((es) => es.map((e) => e.id))
  const before = await order('Pages')
  const pages = region.getByRole('listbox', { name: 'Pages' })
  const band = async (from: [number, number], to: Locator) => {
    const b = (await to.boundingBox())!
    await page.mouse.move(...from)
    await page.mouse.down()
    await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 4 })
    await page.mouse.up()
  }
  const corner = async (name: string) => {
    const b = (await region.getByRole('listbox', { name }).locator('xpath=..').boundingBox())!
    return [b.x + b.width - 12, b.y + b.height - 12] as [number, number]
  }
  await band(await corner('Pages'), option(page, 4))
  await expect(option(page, 4)).toHaveAttribute('aria-selected', 'true')
  await expect(option(page, 1)).toHaveAttribute('aria-selected', 'false')
  await option(page, 2).click({ modifiers: ['Control'] })
  await option(page, 3).click({ modifiers: ['Shift'] })
  await expect(region).toBeVisible()
  await expect(pages.getByRole('option', { selected: true })).toHaveCount(2)
  await expect(region.getByText('1', { exact: true })).toHaveCSS('user-select', 'none')
  const ghost = await option(page, 2).evaluate((el) => {
    el.dispatchEvent(new DragEvent('dragstart', { bubbles: true, dataTransfer: new DataTransfer() }))
    const cards = [...document.querySelectorAll<HTMLCanvasElement>('.ov-ghost canvas')]
    const c = cards.at(-1)!
    const d = c.getContext('2d')!.getImageData(0, 0, c.width, c.height).data
    const drawn = d.some((v, i) => i % 4 === 3 && v > 0)
    el.dispatchEvent(new DragEvent('dragend', { bubbles: true }))
    return [cards.length, drawn, document.querySelector('.ov-ghost b')?.textContent]
  })
  expect(ghost).toEqual([2, true, '2'])
  await option(page, 2).dragTo(option(page, 5), { targetPosition: { x: 10, y: 10 } })
  await expect.poll(() => order('Pages')).toEqual([before[0], before[3], before[1], before[2], before[4]])
  await page.keyboard.press('Control+z')
  await expect.poll(() => order('Pages')).toEqual(before)

  const masters = await order('Masters')
  const master = (name: string) => region.getByRole('option', { name, exact: true })
  await band(await corner('Masters'), master('B-Master'))
  await expect(region.getByRole('listbox', { name: 'Masters' }).getByRole('option', { selected: true })).toHaveCount(2)
  await expect(pages.getByRole('option', { selected: true })).toHaveCount(0)
  await master('B-Master').dragTo(master('A-Master'), { targetPosition: { x: 10, y: 2 } })
  await expect.poll(() => order('Masters')).toEqual([masters[1], masters[2], masters[0]])
  await master('A-Master').click()
  await expect(region).toHaveCount(0)
  await expect(page.getByText(/Editing master/)).toContainText('A-Master')
})
