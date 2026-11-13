import { type Page } from '@playwright/test'
import { expect, test, assets, choose, open, pixels, screen } from './util'

const fonts = (page: Page) => page.getByRole('region', { name: 'Fonts' })

async function add(page: Page, files: string | { name: string; mimeType: string; buffer: Buffer }) {
  await assets(page)
  const chooser = page.waitForEvent('filechooser')
  await fonts(page).getByRole('button', { name: 'Add font' }).click()
  await page.getByRole('menuitem', { name: 'Font files…' }).click()
  await (await chooser).setFiles(files)
}

test('an added woff2 font is listed, stays after a reload and is removed', async ({ page }) => {
  await open(page)
  await assets(page)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
  await add(page, new URL('../../engine/fonts/DMMono-Regular.woff2', import.meta.url).pathname)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await page.reload()
  await assets(page)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await expect(page.getByRole('dialog', { name: 'New document' })).toHaveCount(0)
  await fonts(page).getByRole('listitem').last().hover()
  await fonts(page).getByRole('button', { name: 'Remove DM Mono Regular' }).click()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
  await page.reload()
  await assets(page)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
})

test('text is set in a font picked in the text section', async ({ page }) => {
  await open(page)
  await add(page, new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4', 'DM Mono Regular'])
  await page.getByRole('tab', { name: 'Layers' }).click()
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
    page.getByText('Could not add notes.ttf: not a TrueType or OpenType font. Choose a .ttf, .otf, .woff or .woff2 file.'),
  ).toBeVisible()
  await expect(fonts(page).getByRole('listitem')).toHaveText(['Source Serif 4'])
})

test('a text style sets the font of its text', async ({ page }) => {
  await open(page)
  await add(page, new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await page.getByRole('tab', { name: 'Layers' }).click()
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await panel.getByTitle('Text style', { exact: true }).first().click()
  await page.getByRole('menuitem', { name: 'New text style' }).click()
  await page.keyboard.press('Escape')
  const left = await assets(page)
  await left.getByRole('button', { name: 'Edit Text style 1' }).click()
  await choose(left.getByRole('group', { name: 'Text style 1' }).getByRole('combobox', { name: 'Font', exact: true }), 'DM Mono')
  await page.getByRole('tab', { name: 'Layers' }).click()
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  await expect(panel.getByRole('combobox', { name: 'Font', exact: true })).toHaveText('DM Mono')
  await expect(panel.getByTitle('Text style', { exact: true })).toContainText('Text style 1')
})

test('add font at the end of the font select adds a font file', async ({ page }) => {
  await open(page)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const font = page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Font', exact: true })
  await font.click()
  const chooser = page.waitForEvent('filechooser')
  await page.getByRole('menuitemradio', { name: 'Add font…' }).click()
  await (await chooser).setFiles(new URL('../../engine/fonts/DMMono-Regular.ttf', import.meta.url).pathname)
  await font.click()
  await expect(page.getByRole('menuitemradio', { name: 'DM Mono' })).toBeVisible()
})

test('the axes of a variable font are set in the type options and drawn', async ({ page }) => {
  await open(page)
  await add(page, new URL('../src/fonts/hanken-grotesk-latin.woff2', import.meta.url).pathname)
  await page.getByRole('tab', { name: 'Layers' }).click()
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const options = page.getByRole('dialog', { name: 'Type options' })
  await panel.getByRole('button', { name: 'Type options' }).click()
  await expect(options.getByRole('tab', { name: 'Variable' })).toHaveCount(0)
  await page.keyboard.press('Escape')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  await choose(panel.getByRole('combobox', { name: 'Font', exact: true }), 'Hanken Grotesk')
  const [[x0, y0], [x1, y1]] = [await screen(page, 15, 95), await screen(page, 133, 125)]
  const ink = async () => (await pixels(page, x0, y0, x1 - x0, y1 - y0)).filter(([r, g, b]) => r + g + b < 384).length
  const light = await ink()
  await panel.getByRole('button', { name: 'Type options' }).click()
  await options.getByRole('tab', { name: 'Variable' }).click()
  const weight = options.getByRole('textbox', { name: 'Weight' })
  await expect(weight).toHaveValue('400')
  await weight.fill('900')
  await weight.press('Enter')
  await expect(weight).toHaveValue('900')
  await page.keyboard.press('Escape')
  await expect.poll(ink).toBeGreaterThan(light * 1.3)
  await page.keyboard.press('Control+z')
  await expect.poll(ink).toBeLessThan(light * 1.1)
})
