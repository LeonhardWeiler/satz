import { expect, test, type Page } from '@playwright/test'
import { open } from './util'

const fonts = (page: Page) => page.getByRole('region', { name: 'Fonts' })

async function add(page: Page, files: string | { name: string; mimeType: string; buffer: Buffer }) {
  const chooser = page.waitForEvent('filechooser')
  await fonts(page).getByRole('button', { name: 'Add font' }).click()
  await (await chooser).setFiles(files)
}

test('an added font is listed and stays after a reload', async ({ page }) => {
  await open(page)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
  await add(page, new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await page.reload()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
})

test('a file that is not a font is not added and says why', async ({ page }) => {
  await open(page)
  await add(page, { name: 'notes.ttf', mimeType: 'font/ttf', buffer: Buffer.from('notes') })
  await expect(
    page.getByText('Could not add notes.ttf: not a TrueType or OpenType font. Choose a .ttf or .otf file.'),
  ).toBeVisible()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
})
