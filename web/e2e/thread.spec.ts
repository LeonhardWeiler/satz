import { expect, test, type Page } from '@playwright/test'
import { PAIR, STORY, frameOnNewPage, open, pixels, port, screen } from './util'

const overset = async (page: Page, at: [number, number]) =>
  (await pixels(page, at[0] - 4, at[1] - 4, 9, 9)).some(([r, g, b]) => r > 200 && g < 110 && b < 110)

test('text threads from the out-port into a clicked frame and a new one, across pages, and the caret follows', async ({ page }) => {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const canvas = page.getByLabel('Page canvas')
  const title = page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })
  const a = [20, 20, 60, 40]
  const b = [80, 20, 120, 40]

  await frameOnNewPage(page, a)
  await page.keyboard.type(STORY)
  await page.keyboard.press('Escape')
  await expect(title).toHaveText(STORY.slice(0, 40))
  await page.mouse.move(1, 1)
  expect(await overset(page, await port(page, a, true))).toBe(true)

  await page.mouse.click(...(await port(page, a, true)))
  await expect(canvas).toHaveAttribute('data-threading', '')
  await page.mouse.click(...(await screen(page, b[0], b[1])))
  await expect(canvas).not.toHaveAttribute('data-threading')
  await expect(layers.getByRole('treeitem')).toHaveCount(2)
  await expect(title).not.toHaveText(/^Lorem/)
  const second = (await title.textContent())!
  expect(STORY).toContain(second.slice(0, 20))
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await expect(panel.getByRole('radio', { name: 'Auto width' })).toBeDisabled()
  await expect(panel.getByRole('radio', { name: 'Auto height' })).toBeEnabled()

  // Page 3 shows beside page 2 in their spread.
  await frameOnNewPage(page, a, PAIR, 1)
  await page.keyboard.type('Tail')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.mouse.click(...(await screen(page, 100, 30, PAIR, 0)))
  await expect(pages.getByRole('button', { name: 'Page 2', exact: true })).toHaveAttribute('aria-current', 'page')
  await expect(title).toHaveText(second)
  await page.mouse.click(...(await port(page, b, true, PAIR, 0)))
  await page.mouse.click(...(await screen(page, 40, 30, PAIR, 1)))
  await expect(title).not.toHaveText('Text')

  await page.mouse.dblclick(...(await screen(page, 40, 25, PAIR, 1)))
  await expect(page.getByRole('textbox', { name: 'Text editor' })).toBeFocused()
  await page.keyboard.press('Control+Home')
  await expect(pages.getByRole('button', { name: 'Page 2', exact: true })).toHaveAttribute('aria-current', 'page')
  await page.keyboard.type('X')
  await expect(layers.getByRole('button', { name: `X${STORY.slice(0, 39)}`, exact: true })).toBeVisible()
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')

  await page.mouse.click(...(await screen(page, 100, 30, PAIR, 0)))
  await page.mouse.dblclick(...(await port(page, b, true, PAIR, 0)))
  await pages.getByRole('button', { name: 'Page 3', exact: true }).click()
  await expect(layers.getByRole('button', { name: 'Text', exact: true })).toBeVisible()
  await page.keyboard.press('Control+z')
  await expect(layers.getByRole('button', { name: 'Text', exact: true })).toHaveCount(0)
})
