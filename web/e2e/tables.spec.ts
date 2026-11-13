import { expect, test, addPage, drag, open, screen } from './util'

test('a drawn table takes text cell by cell, grows a row past its end, merges and splits', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  const table = panel.getByRole('region', { name: /^Table \d/ })
  const menu = async (button: string, item: string) => {
    await table.getByRole('button', { name: button }).click()
    await page.getByRole('menuitem', { name: item }).click()
  }
  await addPage(page)
  await page.getByRole('button', { name: 'Table', exact: true }).click()
  await drag(page, await screen(page, 20, 20), await screen(page, 110, 50))
  await expect(layers.getByRole('button', { name: 'C2', exact: true })).toBeVisible()
  await menu('Insert rows or columns', 'Insert row below')
  await expect(layers.getByRole('button', { name: 'C3', exact: true })).toBeVisible()

  await page.mouse.dblclick(...(await screen(page, 25, 23)))
  const editor = page.getByRole('textbox', { name: 'Text editor' })
  await expect(editor).toBeFocused()
  await page.keyboard.type('a')
  await page.keyboard.press('Tab')
  await page.keyboard.type('b')
  for (let i = 0; i < 11; i++) await page.keyboard.press('Tab')
  await page.keyboard.type('z')
  await page.keyboard.press('Escape')
  for (const name of ['a', 'b', 'z']) await expect(layers.getByRole('button', { name, exact: true })).toBeVisible()
  await menu('Cell options', 'Delete rows')
  await expect(layers.getByRole('button', { name: 'z', exact: true })).toHaveCount(0)

  await layers.getByRole('button', { name: 'a', exact: true }).click()
  await layers.getByRole('button', { name: 'b', exact: true }).click({ modifiers: ['Shift'] })
  await menu('Cell options', 'Merge cells')
  await expect(layers.getByRole('button', { name: 'b', exact: true })).toHaveCount(0)
  await menu('Cell options', 'Split cell')
  await expect(layers.getByRole('button', { name: 'B1', exact: true })).toBeVisible()
  await expect(table.getByRole('textbox', { name: 'Column width' })).toBeVisible()
})
