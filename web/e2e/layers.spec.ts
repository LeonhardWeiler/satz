import { execFileSync } from 'node:child_process'
import { expect, test, type Page } from '@playwright/test'
import { addMaster, colors, drag, open, screen, showPage } from './util'

const layers = (page: Page) => page.getByRole('tree', { name: 'Layers' })
const rect = (page: Page) => layers(page).getByRole('treeitem', { name: 'Rectangle' }).first()
const at = async (page: Page, p: readonly [number, number]) => (await colors(page, [p]))[0].join()

async function exported(page: Page) {
  const download = page.waitForEvent('download')
  await page.getByRole('button', { name: 'Export', exact: true }).click()
  const pdf = (await (await download).path())!
  return execFileSync('mutool', ['draw', '-F', 'trace', '-o', '-', pdf]).toString().match(/<fill_path/g)?.length ?? 0
}

/** Draws a rectangle short of the bleed at the top of the page and returns a point inside it. */
async function drawn(page: Page) {
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, -1), await screen(page, 40, 20))
  await expect(rect(page)).toHaveAttribute('aria-selected', 'true')
  return screen(page, 30, 10)
}

test('a hidden layer is not on the canvas, in the pdf or in preflight until shown again', async ({ page }) => {
  await open(page)
  await page.mouse.move(1, 1)
  const paper = await at(page, await screen(page, 30, 10))
  const inside = await drawn(page)
  const short = page.getByRole('region', { name: 'Preflight' }).getByRole('button', { name: /Short of the bleed/ })
  await expect(short).toHaveCount(1)
  const before = await exported(page)
  await page.keyboard.press('Escape')
  await page.mouse.move(1, 1)
  expect(await at(page, inside)).not.toBe(paper)

  await rect(page).hover()
  await rect(page).getByRole('button', { name: 'Hide' }).click()
  await page.mouse.move(1, 1)
  const show = rect(page).getByRole('button', { name: 'Show' })
  await expect(show).toBeVisible()
  expect(await at(page, inside)).toBe(paper)
  await expect(short).toHaveCount(0)
  expect(await exported(page)).toBe(before - 1)
  await page.mouse.click(...inside)
  await expect(rect(page)).toHaveAttribute('aria-selected', 'false')

  await show.click()
  await page.mouse.move(1, 1)
  expect(await at(page, inside)).not.toBe(paper)
  await expect(short).toHaveCount(1)
})

test('a locked layer is not hit on the canvas but is selected in the tree', async ({ page }) => {
  await open(page)
  const inside = await drawn(page)
  await expect(rect(page)).toHaveAttribute('aria-selected', 'true')
  await page.keyboard.press('Control+Shift+L')
  await expect(rect(page).getByRole('button', { name: 'Unlock' })).toBeVisible()
  await page.keyboard.press('Escape')
  await page.mouse.click(...inside)
  await expect(rect(page)).toHaveAttribute('aria-selected', 'false')
  await drag(page, await screen(page, 10, -5), await screen(page, 50, 30))
  await expect(rect(page)).toHaveAttribute('aria-selected', 'false')

  await rect(page).getByRole('button', { name: 'Rectangle', exact: true }).click()
  await expect(rect(page)).toHaveAttribute('aria-selected', 'true')
  await page.keyboard.press('Control+Shift+H')
  await expect(rect(page).getByRole('button', { name: 'Show' })).toBeVisible()
  await page.keyboard.press('Control+Z')
  await expect(rect(page).getByRole('button', { name: 'Show' })).toHaveCount(0)
  await page.keyboard.press('Control+Shift+L')
  await page.keyboard.press('Escape')
  await page.mouse.click(...inside)
  await expect(rect(page)).toHaveAttribute('aria-selected', 'true')
})

test('a hidden master layer is not drawn on its pages', async ({ page }) => {
  await open(page)
  await page.mouse.move(1, 1)
  const paper = await at(page, await screen(page, 40, 202))
  await addMaster(page)
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 197), await screen(page, 60, 207))
  await rect(page).hover()
  await rect(page).getByRole('button', { name: 'Hide' }).click()
  await showPage(page, 1)
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' }).selectOption('A-Master')
  await expect(page.getByText('A-Master master items')).toBeVisible()
  await page.mouse.move(1, 1)
  expect(await at(page, await screen(page, 40, 202))).toBe(paper)
})

test('hovering a layer row highlights it on the canvas', async ({ page }) => {
  await open(page)
  const canvas = page.getByLabel('Page canvas')
  const before = await canvas.screenshot()
  await layers(page).getByRole('button', { name: 'Sun' }).hover()
  await expect.poll(async () => (await canvas.screenshot()).equals(before)).toBe(false)
  await page.mouse.move(1, 1)
  await expect.poll(async () => (await canvas.screenshot()).equals(before)).toBe(true)
})

test('the keyboard walks the layers: tab siblings, enter children, shift enter parent, f2 renames', async ({ page }) => {
  await open(page)
  const selected = layers(page).getByRole('treeitem', { selected: true })
  const names = layers(page).locator('.layer-name')
  await page.getByLabel('Page canvas').focus()
  await page.keyboard.press('Tab')
  await expect(selected).toHaveCount(1)
  await expect(selected.locator('.layer-name').first()).toHaveText((await names.first().textContent())!)
  await page.keyboard.press('Shift+Tab')
  await page.keyboard.press('Tab')
  await expect(selected.locator('.layer-name').first()).toHaveText((await names.first().textContent())!)

  await layers(page).getByRole('button', { name: 'Shapes' }).click()
  await page.getByLabel('Page canvas').focus()
  await page.keyboard.press('Enter')
  await expect(layers(page).getByRole('treeitem', { name: 'Shapes' })).toHaveAttribute('aria-selected', 'false')
  const kids = await selected.count()
  expect(kids).toBeGreaterThan(1)
  await page.keyboard.press('Shift+Enter')
  await expect(layers(page).getByRole('treeitem', { name: 'Shapes' })).toHaveAttribute('aria-selected', 'true')

  await page.keyboard.press('F2')
  await layers(page).getByRole('textbox', { name: 'Layer name' }).fill('Forms')
  await page.keyboard.press('Enter')
  await expect(layers(page).getByRole('button', { name: 'Forms' })).toBeVisible()

  await layers(page).getByRole('button', { name: 'Forms' }).focus()
  await page.keyboard.press('Enter')
  await expect(selected).toHaveCount(kids)
  await layers(page).getByRole('button', { name: 'Sun' }).focus()
  await page.keyboard.press('Shift+Enter')
  await expect(layers(page).getByRole('treeitem', { name: 'Forms' })).toHaveAttribute('aria-selected', 'true')
})

test('an empty page says how to draw', async ({ page }) => {
  await open(page)
  await addMaster(page)
  await expect(layers(page).getByRole('treeitem')).toHaveCount(0)
  await expect(page.getByText('Draw with R, O or T')).toBeVisible()
})
