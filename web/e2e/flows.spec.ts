import { type Page } from '@playwright/test'
import { expect, test, addMaster, autosaved, current, option, overview } from './util'

const start = (page: Page) => page.getByRole('dialog', { name: 'New document' })

async function a5Booklet(page: Page) {
  await page.goto('')
  const dialog = start(page)
  await dialog.getByRole('radio', { name: /A5/ }).click()
  await dialog.getByRole('textbox', { name: 'Pages' }).fill('8')
  await dialog.getByRole('textbox', { name: 'Pages' }).press('Tab')
  await dialog.getByRole('checkbox', { name: 'Facing pages' }).check()
  await dialog.getByRole('radio', { name: /A5/ }).press('Enter')
  await expect(dialog).toHaveCount(0)
}

test('a first visit creates an a5 booklet, a reload opens it and ctrl+n opens a file', async ({ page }) => {
  page.on('filechooser', () => {})
  await a5Booklet(page)
  const spreads = page.getByRole('navigation', { name: 'Spreads' })
  await expect(spreads.getByRole('button', { name: /^Page / })).toHaveCount(1)
  await expect(current(page, 1)).toBeVisible()
  await page.keyboard.press('PageDown')
  await expect(spreads.getByRole('button', { name: /^Page / })).toHaveText(['2', '3'])

  await autosaved(page, false)
  await page.reload()
  await expect(page.getByLabel('Zoom')).not.toHaveText('0%')
  await expect(start(page)).toHaveCount(0)
  await expect(current(page, 1)).toBeVisible()

  await page.evaluate(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
  await page.keyboard.press('Control+n')
  const chooser = page.waitForEvent('filechooser')
  await start(page).getByRole('button', { name: /Open file/ }).click()
  await (await chooser).setFiles(`${import.meta.dirname}/../../examples/poster.satz`)
  await expect(page).toHaveTitle('poster.satz — Satz')
})

test('the overview lists masters above vertical spreads, assigns a master and reorders by drag', async ({ page }) => {
  await a5Booklet(page)
  await addMaster(page)
  await page.keyboard.press('Escape')
  await addMaster(page)
  await page.keyboard.press('Escape')
  const region = await overview(page)
  const box = async (name: string) => (await region.getByRole('group', { name, exact: true }).boundingBox())!
  const masters = await box('Masters')
  const [one, two, four] = [await box('Spread 1'), await box('Spread 2–3'), await box('Spread 4–5')]
  expect(masters.y + masters.height).toBeLessThanOrEqual(one.y)
  expect(one.y).toBeLessThan(two.y)
  expect(two.y).toBeLessThan(four.y)
  expect(two.x).toBe(four.x)
  expect((await option(page, 1).boundingBox())!.x).toBeGreaterThan((await option(page, 2).boundingBox())!.x)

  const master = page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' })
  await option(page, 3).click()
  await master.selectOption('A-Master')
  await option(page, 4).click({ modifiers: ['Shift'] })
  await expect(master.locator('option:checked')).toHaveText('Mixed')
  await master.selectOption('B-Master')
  await expect(master.locator('option:checked')).toHaveText('B-Master')

  await option(page, 6).dragTo(option(page, 2), { targetPosition: { x: 20, y: 2 } })
  await option(page, 3).click()
  await expect(master.locator('option:checked')).not.toHaveText('B-Master')
  await option(page, 4).click()
  await expect(master.locator('option:checked')).toHaveText('B-Master')
  await option(page, 5).click()
  await expect(master.locator('option:checked')).toHaveText('B-Master')
  await page.keyboard.press('Escape')
  await expect(region).toHaveCount(0)
})

