import { expect, test, frameOnNewPage, open } from './util'

test('a table of contents lists the paragraphs of the chosen text styles and updates', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  await layers.getByRole('button', { name: /^Satz sets type/ }).click()
  await panel.getByTitle('Text style', { exact: true }).first().click()
  await page.getByRole('menuitem', { name: 'New text style' }).click()
  await page.keyboard.press('Escape')

  await frameOnNewPage(page, [20, 20, 120, 60])
  await page.keyboard.type('x')
  await page.keyboard.press('Escape')
  const contents = panel.getByRole('region', { name: 'Table of contents' })
  await contents.getByRole('button', { name: 'Add table of contents' }).click()
  const toc = layers.locator('[aria-selected=true] .layer-name').first()
  await expect(toc).toHaveText(/^Satz sets type/)
  await expect(contents.getByRole('checkbox', { name: 'Text style 1' })).toBeChecked()

  await contents.getByRole('checkbox', { name: 'Text style 1' }).click()
  await expect(contents.getByRole('button', { name: 'Add table of contents' })).toBeVisible()
  await expect(toc).toHaveText(/^Satz sets type/)
})
