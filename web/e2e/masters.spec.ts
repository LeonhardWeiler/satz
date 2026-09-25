import { execFileSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { expect, test, type Page } from '@playwright/test'
import { colors, drag, open, screen } from './util'

const gray = ([r, g, b]: number[]) => [r, g, b].every((c) => Math.abs(c - 0xd9) < 8)

const title = (page: Page) => page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })
const rects = (page: Page) => page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
/** Where the master rectangle shows on page 1, and a point on the page beside it. */
const inside = (page: Page) => screen(page, 40, 202)
const outside = (page: Page) => screen(page, 80, 202)

/** A master with a rectangle at the foot of its right page, applied to page 1. */
async function applied(page: Page) {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 197), await screen(page, 60, 207))
  await expect(rects(page)).toHaveCount(1)
  await pages.getByRole('button', { name: 'Page 1', exact: true }).click()
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' }).selectOption('A-Master')
  await expect(pages.getByRole('listitem').filter({ hasText: 'Page 1' })).toContainText('A')
}

/** `applied`, with the master rectangle overridden on page 1 by ctrl+shift+click. */
async function overridden(page: Page) {
  await applied(page)
  await page.keyboard.down('Control')
  await page.keyboard.down('Shift')
  await page.mouse.click(...(await inside(page)))
  await page.keyboard.up('Shift')
  await page.keyboard.up('Control')
  await expect(title(page)).toHaveText('Rectangle')
}

test('a new master is shown empty and is renamed by a double click', async ({ page }) => {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  const master = pages.getByRole('button', { name: 'A-Master', exact: true })
  await expect(master).toHaveAttribute('aria-current', 'page')
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem')).toHaveCount(0)
  await master.dblclick()
  await pages.getByRole('textbox', { name: 'Master name' }).fill('Body')
  await pages.getByRole('textbox', { name: 'Master name' }).press('Enter')
  await expect(pages.getByRole('button', { name: 'Body', exact: true })).toBeVisible()
})

test('a page draws the layers of its master under its own, which a click does not pick', async ({ page }) => {
  await applied(page)
  await expect(rects(page)).toHaveCount(2)
  await page.mouse.move(1, 1)
  const shown = await colors(page, [await inside(page), await outside(page)])
  expect(shown.map(gray)).toEqual([true, false])
  await page.mouse.click(...(await inside(page)))
  await expect(title(page)).toHaveText('Page')
})

test('ctrl+shift+click overrides a master layer with a copy on the page', async ({ page }) => {
  await overridden(page)
  await expect(rects(page)).toHaveCount(3)
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Reset to master' })).toBeVisible()
})

test('reset to master removes the override, and undo brings it back', async ({ page }) => {
  await overridden(page)
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Reset to master' }).click()
  await expect(rects(page)).toHaveCount(2)
  await expect(title(page)).toHaveText('Page')
  await page.keyboard.press('Control+z')
  await expect(rects(page)).toHaveCount(3)
})

test('a page shows the prefix of its master as its page numbers do, in any script', async ({ page }) => {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  await pages.getByRole('button', { name: 'A-Master', exact: true }).dblclick()
  await pages.getByRole('textbox', { name: 'Master name' }).fill('ÄB-Master')
  await pages.getByRole('textbox', { name: 'Master name' }).press('Enter')
  await pages.getByRole('button', { name: 'Page 1', exact: true }).click()
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' }).selectOption('ÄB-Master')
  await expect(pages.getByTitle('Master ÄB-Master')).toHaveText('ÄB')
})

/** A master spread with a rectangle at the outer foot of each page, applied to pages 2 and 3. */
async function sides(page: Page) {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  await page.keyboard.press('r')
  await drag(page, await screen(page, 10, 190, 0), await screen(page, 30, 205, 0))
  await page.keyboard.press('r')
  await drag(page, await screen(page, 118, 190), await screen(page, 138, 205))
  await expect(rects(page)).toHaveCount(2)
  for (let i = 0; i < 2; i++) {
    await pages.getByRole('button', { name: 'Add page' }).click()
    await panel.getByRole('combobox', { name: 'Master' }).selectOption('A-Master')
  }
}

test('left pages show the left page of a master spread and right pages its right page', async ({ page }) => {
  await sides(page)
  await page.mouse.move(1, 1)
  const at = (i: number, x: number) => screen(page, x, 197, i)
  const shown = await colors(page, [await at(0, 20), await at(0, 128), await at(1, 20), await at(1, 128)])
  expect(shown.map(gray)).toEqual([true, false, false, true])
})

test('a layer of the left master page is overridden in its place on a left page', async ({ page }) => {
  await sides(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await page.keyboard.down('Control')
  await page.keyboard.down('Shift')
  await page.mouse.click(...(await screen(page, 20, 197, 0)))
  await page.keyboard.up('Shift')
  await page.keyboard.up('Control')
  await expect(page.getByRole('navigation', { name: 'Pages' }).getByRole('button', { name: 'Page 2', exact: true })).toHaveAttribute('aria-current', 'page')
  await expect(rects(page)).toHaveCount(1)
  await expect(panel.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')).toHaveValue('10')
  await panel.getByRole('button', { name: 'Reset to master' }).click()
  await expect(rects(page)).toHaveCount(0)
})

test('with facing pages a one-page master gets its layers on both of its pages', async ({ page }) => {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  const facing = page.getByRole('complementary', { name: 'Properties' }).getByRole('checkbox', { name: 'Facing pages' })
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  await facing.uncheck()
  await pages.getByRole('button', { name: 'Add master' }).click()
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 20), await screen(page, 60, 40))
  await page.keyboard.press('Escape')
  await facing.check()
  await expect(rects).toHaveCount(2)
  await page.mouse.move(1, 1)
  const shown = await colors(page, [await screen(page, 40, 30, 0), await screen(page, 40, 30)])
  expect(shown.map(gray)).toEqual([true, true])
  await facing.uncheck()
  await expect(rects).toHaveCount(1)
})

test('a page number on the pages of a master spread shows the number of each page', async ({ page }) => {
  await open(page)
  const pages = page.getByRole('navigation', { name: 'Pages' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  await pages.getByRole('button', { name: 'Add master' }).click()
  await page.keyboard.press('t')
  await page.mouse.click(...(await screen(page, 10, 200, 0)))
  await panel.getByRole('button', { name: 'Insert page number' }).click()
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.keyboard.press('t')
  await page.mouse.click(...(await screen(page, 130, 200)))
  await page.keyboard.press('Control+Alt+Shift+N')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await expect(layers.getByRole('button', { name: '#', exact: true })).toHaveCount(2)
  for (let i = 0; i < 2; i++) {
    await pages.getByRole('button', { name: 'Add page' }).click()
    await panel.getByRole('combobox', { name: 'Master' }).selectOption('A-Master')
  }
  const pdf = join(mkdtempSync(join(tmpdir(), 'satz-')), 'satz.pdf')
  const download = page.waitForEvent('download')
  await panel.getByRole('button', { name: 'Export PDF' }).click()
  await (await download).saveAs(pdf)
  const text = execFileSync('mutool', ['draw', '-q', '-F', 'text', '-o', '-', pdf, '2-3']).toString()
  expect(text.replace(/\s+/g, ' ').trim()).toBe('2 3')
})
