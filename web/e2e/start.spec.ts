import { type Page } from '@playwright/test'
import { expect, test, open, pageCount } from './util'

const start = (page: Page) => page.getByRole('dialog', { name: 'New document' })
const spreads = (page: Page) => page.getByRole('navigation', { name: 'Spreads' })

test('the start screen shows without an autosave and escape opens the booklet', async ({ page }) => {
  await page.goto('')
  await expect(start(page).getByText('Esc opens the booklet')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(start(page)).toHaveCount(0)
  await expect(page).toHaveTitle('booklet.satz - Satz')
  await pageCount(page, 8)
})

test('an example that does not load says so and keeps the start screen', async ({ page }) => {
  test.info().annotations.push({ type: 'errors' })
  await page.route('**/booklet-*.satz', (r) => r.fulfill({ status: 404 }))
  await page.goto('')
  await expect(start(page).getByText('Esc opens the booklet')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.locator('.status')).toHaveText(/Could not open booklet.satz: 404/)
  await expect(start(page)).toBeVisible()
})

test('enter creates a document of the chosen format, orientation and pages', async ({ page }) => {
  await open(page)
  await page.keyboard.press('Control+Alt+n')
  const dialog = start(page)
  await expect(dialog.getByRole('radio', { name: /A5/ })).toBeFocused()
  await page.keyboard.press('ArrowLeft')
  await expect(dialog.getByRole('radio', { name: /A4/ })).toBeFocused()
  await dialog.getByRole('radio', { name: 'Landscape' }).click()
  await dialog.getByRole('textbox', { name: 'Pages' }).fill('3')
  await dialog.getByRole('textbox', { name: 'Pages' }).press('Enter')
  await expect(dialog.getByRole('button', { name: /Create A4, 3 pages/ })).toBeVisible()
  await dialog.getByRole('radio', { name: /A4/ }).press('Enter')
  await expect(dialog).toHaveCount(0)
  await expect(page.getByText('Created A4, 3 pages')).toBeVisible()
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveValue('297')
  await expect(panel.getByRole('textbox', { name: 'H in mm' })).toHaveValue('210')
  await expect(panel.getByRole('combobox', { name: 'Spreads' })).toHaveText('Facing pages')
  await expect(spreads(page).getByRole('button', { name: 'Page 1' })).toBeVisible()
})

test('a custom format takes its width and height', async ({ page }) => {
  await open(page)
  await page.keyboard.press('Control+Alt+n')
  const dialog = start(page)
  await dialog.getByRole('radio', { name: /Custom/ }).click()
  await dialog.getByRole('textbox', { name: 'Width' }).fill('100')
  await dialog.getByRole('textbox', { name: 'Width' }).press('Tab')
  await dialog.getByRole('textbox', { name: 'Pages' }).fill('1')
  await dialog.getByRole('textbox', { name: 'Pages' }).press('Tab')
  await expect(dialog.getByRole('checkbox', { name: 'Facing pages' })).toBeDisabled()
  await dialog.getByRole('button', { name: /Create Custom, 1 page/ }).click()
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByRole('textbox', { name: 'W in mm' })).toHaveValue('100')
})

test('escape returns to the document and an example asks before it discards changes', async ({ page }) => {
  await open(page)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Sun' }).click()
  await page.keyboard.press('Delete')
  await page.keyboard.press('Control+Alt+n')
  await expect(start(page).getByText('Esc returns to the document')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(start(page)).toHaveCount(0)
  await expect(page).toHaveTitle('* Untitled.satz - Satz')

  await page.keyboard.press('Control+Alt+n')
  await start(page).getByRole('button', { name: 'Poster' }).click()
  await page.locator('.ask').getByRole('button', { name: 'Cancel' }).click()
  await expect(start(page)).toBeVisible()
  await start(page).getByRole('button', { name: 'Poster' }).click()
  await page.locator('.ask').getByRole('button', { name: 'Discard' }).click()
  await expect(page).toHaveTitle('poster.satz - Satz')
})

test('open file on the start screen asks for a file as ctrl+o does', async ({ page }) => {
  await page.addInitScript(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
  await open(page)
  await page.keyboard.press('Control+Alt+n')
  const chooser = page.waitForEvent('filechooser')
  await start(page).getByRole('button', { name: /Open file/ }).click()
  await chooser
  await expect(start(page)).toHaveCount(0)
})

test('ctrl+o on the start screen opens the file it asks for', async ({ page }) => {
  await page.addInitScript(() => delete (window as { showOpenFilePicker?: unknown }).showOpenFilePicker)
  await open(page)
  await page.keyboard.press('Control+Alt+n')
  const chooser = page.waitForEvent('filechooser')
  await page.keyboard.press('Control+o')
  await (await chooser).setFiles(`${import.meta.dirname}/../../examples/poster.satz`)
  await expect(page).toHaveTitle('poster.satz - Satz')
})
