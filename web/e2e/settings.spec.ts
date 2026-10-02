import { expect, test } from '@playwright/test'
import { autosaved, choose, open } from './util'

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
  await autosaved(page, false)
  await page.reload()
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expect(panel.getByRole('textbox', { name: 'X in in' })).toBeVisible()
  await expect(page.getByRole('toolbar', { name: 'Quick edit' })).toBeHidden()
})

test('a shortcut changed in the shortcut list runs its command and frees the old keys', async ({ page }) => {
  await open(page)
  await page.keyboard.press('?')
  const help = page.getByRole('dialog', { name: 'Keyboard shortcuts' })
  const keys = help.getByRole('button', { name: 'Shortcut for Rectangle tool' })
  await keys.click()
  await page.keyboard.press('q')
  await expect(keys).toHaveText('Q')
  await page.keyboard.press('Escape')

  const tool = page.getByRole('toolbar', { name: 'Tools' })
  await page.keyboard.press('q')
  await expect(tool.getByRole('button', { name: 'Rectangle', exact: true })).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('v')
  await page.keyboard.press('r')
  await expect(tool.getByRole('button', { name: 'Move', exact: true })).toHaveAttribute('aria-pressed', 'true')

  await page.keyboard.press('?')
  await keys.click()
  await page.keyboard.press('Backspace')
  await expect(keys).toHaveText('R')
})

test('the layers of the spread are listed under a heading for each page', async ({ page }) => {
  await open(page)
  await page.getByRole('button', { name: 'Add page' }).click()
  await page.getByRole('button', { name: 'Add page' }).click()
  const tree = page.getByRole('tree', { name: 'Layers' })
  await expect(tree.getByRole('button', { name: 'Page 2' })).toHaveCount(0)

  await page.keyboard.press('Control+Comma')
  await choose(page.getByRole('dialog', { name: 'Settings' }).getByRole('combobox', { name: 'Layers' }), 'Of the spread')
  await page.keyboard.press('Escape')
  await expect(tree.getByRole('button', { name: /^Page \d$/ })).toHaveText(['Page 2', 'Page 3'])
  await tree.getByRole('button', { name: 'Page 2' }).click()
  await expect(tree.getByRole('treeitem', { name: 'Page 2' })).toHaveAttribute('aria-expanded', 'false')
})
