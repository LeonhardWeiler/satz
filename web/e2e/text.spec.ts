import { expect, test } from '@playwright/test'
import { open } from './util'

test('type attributes and a text style are set in the text section and edited on the page', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const text = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ })
  const field = (name: string) => panel.getByRole('textbox', { name })
  const commit = async (name: string, value: string) => {
    await field(name).fill(value)
    await field(name).press('Enter')
  }
  await text.click()
  await expect(field('Font size in pt')).toHaveValue('14')
  await expect(field('Line height in pt')).toHaveValue('Auto')
  await expect(panel.getByRole('radio', { name: 'Justify' })).toBeChecked()

  await commit('Font size in pt', '11')
  await commit('Line height in pt', '15')
  await panel.getByRole('radio', { name: 'Align center' }).click()
  await expect(panel.getByRole('radio', { name: 'Align center' })).toBeChecked()
  await panel.getByRole('button', { name: 'Create text style' }).click()
  await expect(panel.getByRole('combobox', { name: 'Text style' }).locator('option:checked')).toHaveText('Text style 1')

  await page.keyboard.press('Escape')
  const style = panel.getByRole('group', { name: 'Text style 1' })
  await expect(style.getByRole('textbox', { name: 'Font size in pt' })).toHaveValue('11')
  await style.getByRole('textbox', { name: 'Font size in pt' }).fill('9')
  await style.getByRole('textbox', { name: 'Font size in pt' }).press('Enter')
  await text.click()
  await expect(field('Font size in pt')).toHaveValue('9')
  await commit('Line height in pt', 'auto')
  await expect(field('Line height in pt')).toHaveValue('Auto')
  await expect(panel.getByRole('combobox', { name: 'Text style' }).locator('option:checked')).toHaveText('No style')
  await expect(field('Font size in pt')).toHaveValue('9')
})

test('hyphenation is switched per paragraph with its language', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: /^Satz sets type/ }).click()
  const hyphenate = panel.getByRole('checkbox', { name: 'Hyphenate' })
  const lang = panel.getByRole('combobox', { name: 'Hyphenation language' })
  await expect(hyphenate).toBeChecked()
  await expect(lang).toHaveValue('en')
  await lang.selectOption('German')
  await hyphenate.uncheck()
  await page.keyboard.press('Control+z')
  await expect(hyphenate).toBeChecked()
  await expect(lang).toHaveValue('de')
})
