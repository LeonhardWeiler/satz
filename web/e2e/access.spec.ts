import { readFileSync } from 'node:fs'
import { type Page } from '@playwright/test'
import { expect, test, addPage, choose, open, pageCount } from './util'

/** Pickers that hand out files of the origin private file system and count how often they asked. */
async function pickers(page: Page) {
  await page.addInitScript(() => {
    const dir = () => navigator.storage.getDirectory()
    const asked = () => (localStorage.asked = Number(localStorage.asked ?? 0) + 1)
    Object.assign(window, {
      showSaveFilePicker: async ({ suggestedName }: { suggestedName: string }) => {
        asked()
        return (await dir()).getFileHandle(suggestedName, { create: true })
      },
      showOpenFilePicker: async () => {
        asked()
        return [await (await dir()).getFileHandle('Untitled.satz')]
      },
    })
  })
}
const asked = (page: Page) => page.evaluate(() => Number(localStorage.asked))

test('saving asks for a file once and then saves into it again', async ({ page }) => {
  await pickers(page)
  await open(page)
  for (let i = 0; i < 2; i++) {
    await addPage(page)
    await page.keyboard.press('Control+s')
    await expect(page).toHaveTitle('Untitled.satz - Satz')
  }
  expect(await asked(page)).toBe(1)
  const size = await page.evaluate(async () => (await (await (await navigator.storage.getDirectory()).getFileHandle('Untitled.satz')).getFile()).size)
  expect(size).toBeGreaterThan(0)
})

test('a file picked to open replaces the document with it', async ({ page }) => {
  await pickers(page)
  await open(page)
  await addPage(page)
  await page.keyboard.press('Control+s')
  await expect(page).toHaveTitle('Untitled.satz - Satz')
  await addPage(page)
  await page.keyboard.press('Control+o')
  await page.locator('.ask').getByRole('button', { name: 'Discard' }).click()
  await pageCount(page, 2)
  await expect(page).toHaveTitle('Untitled.satz - Satz')
})

test('missing fonts are found among the fonts of this computer', async ({ page }) => {
  const font = readFileSync(new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url)).toString('base64')
  await page.addInitScript((font) => {
    const bytes = Uint8Array.from(atob(font), (c) => c.charCodeAt(0))
    Object.assign(window, { queryLocalFonts: async () => [{ fullName: 'DM Mono Regular', family: 'DM Mono', style: 'Regular', postscriptName: 'DMMono-Regular', blob: async () => new Blob([bytes]) }] })
  }, font)
  await open(page)
  const fonts = page.getByRole('region', { name: 'Fonts' })
  const chooser = page.waitForEvent('filechooser')
  await fonts.getByRole('button', { name: 'Add font' }).click()
  await (await chooser).setFiles(new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  await choose(page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Font', exact: true }), 'DM Mono')
  await page.keyboard.press('Escape')
  await fonts.getByRole('listitem').last().hover()
  await fonts.getByRole('button', { name: 'Remove DM Mono Regular' }).click()
  await expect(fonts.locator('.missing')).toHaveText(['DM Mono Regular'])
  await fonts.getByRole('button', { name: 'Find missing fonts on this computer' }).click()
  await expect(page.getByText('Found 1 of 1 missing fonts on this computer.')).toBeVisible()
  await expect(fonts.locator('.missing')).toHaveCount(0)
})

test('the fonts of this computer are listed and picked like added ones', async ({ page }) => {
  const font = readFileSync(new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url)).toString('base64')
  await page.addInitScript((font) => {
    const bytes = Uint8Array.from(atob(font), (c) => c.charCodeAt(0))
    Object.assign(window, { queryLocalFonts: async () => [{ fullName: 'DM Mono Regular', family: 'DM Mono', style: 'Regular', postscriptName: 'DMMono-Regular', blob: async () => new Blob([bytes]) }] })
  }, font)
  await open(page)
  const fonts = page.getByRole('region', { name: 'Fonts' })
  await fonts.getByRole('button', { name: 'Use local fonts' }).click()
  await expect(fonts.getByRole('button', { name: 'Use local fonts' })).toHaveCount(0)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const select = page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Font', exact: true })
  await select.click()
  await page.keyboard.press('d')
  await expect(page.getByRole('menuitemradio', { name: 'DM Mono' })).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(select).toHaveText('DM Mono')
  await page.keyboard.press('Escape')
  await expect(fonts.getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
})
