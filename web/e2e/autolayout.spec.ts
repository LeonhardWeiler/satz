import { expect, test } from '@playwright/test'
import { drag, open, screen } from './util'

test('shift+a adds a hugging auto layout whose padding, sizing and alignment move the child', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const field = (name: string) => panel.getByRole('textbox', { name })
  const type = async (name: string, v: string) => {
    await field(name).fill(v)
    await field(name).press('Enter')
  }
  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await page.keyboard.press('Shift+A')
  await expect(panel.getByRole('radio', { name: 'Horizontal layout' })).toBeChecked()
  await expect(field('W in mm')).toHaveValue('140')
  await expect(field('H in mm')).toHaveValue('48')
  await expect(panel.getByRole('combobox', { name: 'Width sizing' })).toHaveValue('hug')

  await type('Left padding in mm', '10')
  await expect(field('W in mm')).toHaveValue('70')
  await panel.getByRole('combobox', { name: 'Width sizing' }).selectOption('Fixed')
  await type('W in mm', '100')
  await panel.getByRole('radio', { name: 'Align top right' }).click()

  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expect(field('X in mm')).toHaveValue('55')
  await expect(panel.getByRole('combobox', { name: 'Horizontal constraint' })).toHaveCount(0)
  await panel.getByRole('combobox', { name: 'Width sizing' }).selectOption('Fill')
  await expect(field('W in mm')).toHaveValue('90')
  await expect(field('X in mm')).toHaveValue('25')

  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await page.keyboard.press('Shift+Alt+A')
  await expect(panel.getByRole('radio', { name: 'Horizontal layout' })).toHaveCount(0)
  await page.keyboard.press('Control+z')
  await expect(panel.getByRole('radio', { name: 'Horizontal layout' })).toBeChecked()
})

test('dragging a child of an auto layout frame past its sibling reorders them', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  for (const x of [20, 40]) {
    await page.keyboard.press('r')
    await drag(page, await screen(page, x, 80), await screen(page, x + 10, 90))
  }
  const layers = page.getByRole('tree', { name: 'Layers' })
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).nth(0).click()
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).nth(1).click({ modifiers: ['Shift'] })
  await page.keyboard.press('Shift+A')
  await expect(panel.getByRole('textbox', { name: 'Gap in mm' })).toHaveValue('10')

  await page.keyboard.press('Escape')
  await page.keyboard.down('Control')
  await page.mouse.click(...(await screen(page, 25, 85)))
  await page.keyboard.up('Control')
  await expect(panel.getByRole('textbox', { name: 'X in mm' })).toHaveValue('20')
  await drag(page, await screen(page, 25, 85), await screen(page, 58, 85))
  await expect(panel.getByRole('textbox', { name: 'X in mm' })).toHaveValue('40')
  await page.keyboard.press('Control+z')
  await expect(panel.getByRole('textbox', { name: 'X in mm' })).toHaveValue('20')
})

test('ctrl while resizing a frame leaves its children where they are', async ({ page }) => {
  await open(page)
  const layers = page.getByRole('tree', { name: 'Layers' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await panel.getByRole('combobox', { name: 'Horizontal constraint' }).selectOption('Right')
  await layers.getByRole('button', { name: 'Frame', exact: true }).click()
  await page.keyboard.down('Control')
  await drag(page, await screen(page, 133, 183.5), await screen(page, 143, 183.5))
  await page.keyboard.up('Control')
  await expect(panel.getByRole('textbox', { name: 'W in mm' })).toHaveValue('128')
  await layers.getByRole('button', { name: 'Rectangle', exact: true }).first().click()
  await expect(panel.getByRole('textbox', { name: 'X in mm' })).toHaveValue('95')
})
