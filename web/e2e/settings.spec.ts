import { expect, test } from '@playwright/test'
import { choose, open } from './util'

test('the length unit and the quick edit bar are set in the settings and survive a reload', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  const x = Number(await panel.getByRole('textbox', { name: 'X in mm' }).inputValue())
  await expect(page.getByRole('toolbar', { name: 'Quick edit' })).toBeVisible()

  await page.keyboard.press('Control+Comma')
  const dialog = page.getByRole('dialog', { name: 'Settings' })
  await choose(dialog.getByRole('combobox', { name: 'Unit' }), 'Inches')
  await dialog.getByRole('checkbox', { name: /Quick edit bar/ }).uncheck()
  await page.keyboard.press('Escape')

  await expect(page.getByRole('toolbar', { name: 'Quick edit' })).toBeHidden()
  await expect(panel.getByRole('textbox', { name: 'X in in' })).toHaveValue(String(Math.round((x / 25.4) * 100) / 100))
  await page.reload()
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expect(panel.getByRole('textbox', { name: 'X in in' })).toBeVisible()
  await expect(page.getByRole('toolbar', { name: 'Quick edit' })).toBeHidden()
})
