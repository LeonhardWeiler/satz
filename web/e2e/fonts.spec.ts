import { type Page } from '@playwright/test'
import { expect, test, choose, open } from './util'

const fonts = (page: Page) => page.getByRole('region', { name: 'Fonts' })

async function add(page: Page, files: string | { name: string; mimeType: string; buffer: Buffer }) {
  const chooser = page.waitForEvent('filechooser')
  await fonts(page).getByRole('button', { name: 'Add font' }).click()
  await (await chooser).setFiles(files)
}

test('an added font is listed, stays after a reload and is removed', async ({ page }) => {
  await open(page)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
  await add(page, new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await page.reload()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await expect(page.getByRole('dialog', { name: 'New document' })).toHaveCount(0)
  await fonts(page).getByRole('listitem').last().hover()
  await fonts(page).getByRole('button', { name: 'Remove DM Mono Regular' }).click()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
  await page.reload()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
})

test('text is set in a font picked in the text section', async ({ page }) => {
  await open(page)
  await add(page, new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const font = page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Font', exact: true })
  await expect(font).toHaveText('Source Serif 4')
  await choose(font, 'DM Mono')
  await expect(font).toHaveText('DM Mono')
  await choose(font, 'Source Serif 4')
  await expect(font).toHaveText('Source Serif 4')
  await page.keyboard.press('Control+z')
  await expect(font).toHaveText('DM Mono')
})

test('a file that is not a font is not added and says why', async ({ page }) => {
  await open(page)
  await add(page, { name: 'notes.ttf', mimeType: 'font/ttf', buffer: Buffer.from('notes') })
  await expect(
    page.getByText('Could not add notes.ttf: not a TrueType or OpenType font. Choose a .ttf or .otf file.'),
  ).toBeVisible()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
})
