import { expect, test, type Page } from '@playwright/test'
import { STORY, frameOnNewPage, open, pixels, port, screen } from './util'

const A = [20, 20, 60, 40]
const B = [80, 20, 120, 40]

const overset = async (page: Page, at: readonly [number, number]) =>
  (await pixels(page, at[0] - 4, at[1] - 4, 9, 9)).some(([r, g, b]) => r > 200 && g < 110 && b < 110)
const title = (page: Page) => page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })
const pageButton = (page: Page, n: number) => page.getByRole('navigation', { name: 'Pages' }).getByRole('button', { name: `Page ${n}`, exact: true })

/** Page 2 with the story in frame A, more than fits. */
async function story(page: Page) {
  await open(page)
  await frameOnNewPage(page, A)
  await page.keyboard.type(STORY)
  await page.keyboard.press('Escape')
  await expect(title(page)).toHaveText(STORY.slice(0, 40))
}

/** The story threaded from frame A into frame B on page 2, which is selected; returns B's title. */
async function threaded(page: Page) {
  await story(page)
  await page.mouse.click(...(await port(page, A, true)))
  await page.mouse.click(...(await screen(page, B[0], B[1])))
  await expect(title(page)).not.toHaveText(STORY.slice(0, 40))
  return (await title(page).textContent())!
}

/** Also frame A on page 3, beside page 2 in their spread, threaded on from frame B; returns its title. */
async function acrossPages(page: Page) {
  const second = await threaded(page)
  await frameOnNewPage(page, A)
  await page.keyboard.type('Tail')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.mouse.click(...(await screen(page, 100, 30, 0)))
  await expect(title(page)).toHaveText(second)
  await page.mouse.click(...(await port(page, B, true, 0)))
  await page.mouse.click(...(await screen(page, 40, 30)))
  await expect(title(page)).not.toHaveText(second)
  return { second, third: (await title(page).textContent())! }
}

test('a frame with more text than fits shows a red out-port', async ({ page }) => {
  await story(page)
  await page.mouse.move(1, 1)
  expect(await overset(page, await port(page, A, true))).toBe(true)
})

test('a click on the out-port and then on a frame threads the story on into it', async ({ page }) => {
  const second = await threaded(page)
  const canvas = page.getByLabel('Page canvas')
  await expect(canvas).not.toHaveAttribute('data-threading')
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem')).toHaveCount(2)
  expect(STORY.indexOf(second.slice(0, 20))).toBeGreaterThan(0)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await expect(panel.getByRole('radio', { name: 'Auto width' })).toBeDisabled()
  await expect(panel.getByRole('radio', { name: 'Auto height' })).toBeEnabled()
  await page.keyboard.press('Escape')
  await page.mouse.click(...(await screen(page, A[0] + 5, A[1] + 5)))
  await page.mouse.move(1, 1)
  expect(await overset(page, await port(page, A, true))).toBe(false)
})

test('a thread runs on into a frame on the other page of the spread', async ({ page }) => {
  const { second, third } = await acrossPages(page)
  expect(STORY.indexOf(third.slice(0, 20))).toBeGreaterThan(STORY.indexOf(second.slice(0, 20)))
  await expect(pageButton(page, 3)).toHaveAttribute('aria-current', 'page')
})

test('the caret follows the story back into its first frame on the other page, where typing goes', async ({ page }) => {
  await acrossPages(page)
  await page.mouse.dblclick(...(await screen(page, 40, 25)))
  await expect(page.getByRole('textbox', { name: 'Text editor' })).toBeFocused()
  await page.keyboard.press('Control+Home')
  await expect(pageButton(page, 2)).toHaveAttribute('aria-current', 'page')
  await page.keyboard.type('X')
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: `X${STORY.slice(0, 39)}`, exact: true })).toBeVisible()
})

test('a double click on an out-port unthreads the frames after it, and undo threads them again', async ({ page }) => {
  const { third } = await acrossPages(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  await page.mouse.click(...(await screen(page, 100, 30, 0)))
  await page.mouse.dblclick(...(await port(page, B, true, 0)))
  await pageButton(page, 3).click()
  await expect(layers.getByRole('button', { name: 'Text', exact: true })).toBeVisible()
  await page.keyboard.press('Control+z')
  await expect(layers.getByRole('button', { name: third, exact: true })).toBeVisible()
})
