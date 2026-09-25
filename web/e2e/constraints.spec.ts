import { expect, test } from '@playwright/test'
import { open } from './util'

test('a child pinned right keeps its distance to the right edge when the frame widens', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expect(panel.getByRole('combobox', { name: 'Vertical constraint' })).toHaveValue('min')
  await panel.getByRole('combobox', { name: 'Horizontal constraint' }).selectOption('Right')

  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await expect(panel.getByRole('combobox', { name: 'Horizontal constraint' })).toHaveCount(0)
  await panel.getByRole('textbox', { name: 'W in mm' }).fill('138')
  await panel.getByRole('textbox', { name: 'W in mm' }).press('Enter')

  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expect(panel.getByRole('textbox', { name: 'X in mm' })).toHaveValue('115')
  await expect(panel.getByRole('combobox', { name: 'Horizontal constraint' })).toHaveValue('max')
})
