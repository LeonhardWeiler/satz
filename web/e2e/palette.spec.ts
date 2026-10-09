import { type Page } from '@playwright/test'
import { expect, test, current, open } from './util'

const palette = (page: Page) => page.getByRole('dialog', { name: 'Command palette' })
async function run(page: Page, query: string, name: string | RegExp) {
  await page.keyboard.press('Control+k')
  await palette(page).getByRole('textbox', { name: 'Search' }).fill(query)
  await expect(palette(page).getByRole('option').first()).toHaveText(name)
  await page.keyboard.press('Enter')
  await expect(page.locator('dialog.palette')).toHaveCount(0)
}

test('ctrl k finds and runs commands, layers, pages and text styles', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  await page.keyboard.press('Control+k')
  await expect(palette(page).getByRole('option')).toHaveCount(9)
  await expect(palette(page).getByRole('option', { name: /Export PDF/ })).toContainText('CtrlShiftE')
  await palette(page).getByRole('textbox', { name: 'Search' }).fill('zzqx')
  await expect(palette(page).getByText('Nothing matches. Try a layer name or a command.')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.locator('dialog.palette')).toHaveCount(0)

  await run(page, 'rcttl', /Rectangle toolR$/)
  await expect(page.getByLabel('Page canvas')).toHaveAttribute('data-tool', 'rect')

  await run(page, 'add page', /Add page/)
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await expect(layers.getByRole('treeitem')).toHaveCount(0)

  await run(page, 'sun', /SunPage 1/)
  await expect(current(page, 1)).toHaveAttribute('aria-pressed', 'true')
  await expect(layers.getByRole('treeitem', { name: 'Sun' })).toHaveAttribute('aria-selected', 'true')

  await run(page, 'duplicate page', /Duplicate page/)
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await expect(layers.getByRole('treeitem', { name: 'Sun' })).toBeVisible()
  await page.keyboard.press('Control+z')

  await run(page, 'page 2', /Page 2/)
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await run(page, 'page 1', /Page 1/)

  const panel = page.getByRole('complementary', { name: 'Properties' })
  const style = panel.getByTitle('Text style', { exact: true })
  await layers.getByRole('button', { name: /^Satz sets type/ }).click()
  await panel.getByRole('button', { name: 'Create text style' }).click()
  await page.mouse.move(0, 0)
  await expect(style).toContainText('Text style 1')
  await panel.getByRole('textbox', { name: 'Line height in pt' }).fill('30')
  await panel.getByRole('textbox', { name: 'Line height in pt' }).press('Enter')
  await expect(style).toContainText('Text style 1*')
  await run(page, 'style text', /Style Text style 1/)
  await expect(style).not.toContainText('*')
  await expect(panel.getByRole('textbox', { name: 'Line height in pt' })).toHaveAttribute('placeholder', 'Auto')
})

test('the arrow keys pick a result and a click runs it', async ({ page }) => {
  await open(page)
  await page.keyboard.press('Control+k')
  const selected = palette(page).getByRole('option', { selected: true })
  await expect(selected).toContainText('New document')
  await page.keyboard.press('ArrowDown')
  await expect(selected).toContainText('Open')
  await page.keyboard.press('ArrowUp')
  await page.keyboard.press('ArrowUp')
  await expect(selected).toContainText('New document')
  await palette(page).getByRole('option', { name: /Page overview/ }).click()
  await expect(page.getByRole('region', { name: 'Page overview' })).toBeVisible()
})

test('a swatch fills the selection from the palette and says so when nothing is selected', async ({ page }) => {
  await open(page)
  const swatches = page.getByRole('region', { name: 'Swatches' })
  await swatches.getByRole('button', { name: 'Add swatch' }).click()
  await page.keyboard.press('Escape')
  await expect(swatches.getByRole('option', { name: /Swatch 1/ }).locator('.swatch-ink')).toHaveAttribute('data-ink', /^(RGB|\d+ \d+ \d+ \d+)$/)

  await run(page, 'fill with swatch 1', /Fill with Swatch 1/)
  await expect(page.locator('.status')).toHaveText('Select a layer first')

  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Sun' }).click()
  await run(page, 'fill with swatch 1', /Fill with Swatch 1/)
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByText('Swatch 1')).toBeVisible()
})

test('question mark and the help button show the keyboard shortcuts', async ({ page }) => {
  await open(page)
  const help = page.getByRole('dialog', { name: 'Keyboard shortcuts' })
  await page.keyboard.press('?')
  for (const group of ['Canvas', 'Tools', 'Edit', 'Align', 'Layers', 'Pages', 'View', 'Fields', 'File']) await expect(help.getByRole('region', { name: group })).toBeVisible()
  await expect(help.getByRole('region', { name: 'View' })).toContainText('Command paletteCtrlK')
  await expect(help.getByRole('region', { name: 'Tools' })).toContainText('Pen toolP')
  await help.getByRole('textbox', { name: 'Search shortcuts' }).fill('zoom')
  await expect(help.getByRole('region')).toHaveCount(2)
  await expect(help.getByRole('region', { name: 'View' })).toContainText('Zoom to fit')
  await help.getByRole('textbox', { name: 'Search shortcuts' }).fill('xyz')
  await expect(help).toContainText('No shortcut matches.')
  await page.keyboard.press('Escape')
  await expect(help).toHaveCount(0)
  await page.getByRole('button', { name: 'Keyboard shortcuts' }).click()
  await help.getByRole('button', { name: 'Close' }).click()
  await expect(help).toHaveCount(0)
})
