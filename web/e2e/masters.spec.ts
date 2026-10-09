import { execFileSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { type Page } from '@playwright/test'
import { expect, test, addMaster, addPage, colors, current, drag, exportButton, open, overview, screen, showPage, choose } from './util'

const gray = ([r, g, b]: number[]) => [r, g, b].every((c) => Math.abs(c - 0xd9) < 8)

const title = (page: Page) => page.getByRole('complementary', { name: 'Properties' }).getByRole('heading', { level: 2 })
const rects = (page: Page) => page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
/** Where the master rectangle shows on page 1, and a point on the page beside it. */
const inside = (page: Page) => screen(page, 40, 202)
const outside = (page: Page) => screen(page, 80, 202)

/** A master with a rectangle at the foot of its right page, applied to page 1. */
async function applied(page: Page) {
  await open(page)
  await addMaster(page)
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 197), await screen(page, 60, 207))
  await expect(rects(page)).toHaveCount(1)
  await showPage(page, 1)
  await choose(page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' }), 'A-Master')
  await expect((await overview(page)).getByTitle('Master A-Master', { exact: true })).toHaveText('A')
  await page.keyboard.press('.')
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
  await addMaster(page)
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem')).toHaveCount(0)
  const pages = await overview(page)
  await expect(pages.getByRole('option', { name: 'A-Master', exact: true })).toHaveAttribute('aria-current', 'page')
  await pages.getByText('A-Master', { exact: true }).dblclick()
  await pages.getByRole('textbox', { name: 'Master name' }).fill('Body')
  await pages.getByRole('textbox', { name: 'Master name' }).press('Enter')
  await expect(pages.getByRole('option', { name: 'Body', exact: true })).toBeVisible()
})

test('rename master in the context menu of the page overview renames it', async ({ page }) => {
  await open(page)
  await addMaster(page)
  const pages = await overview(page)
  await pages.getByRole('option', { name: 'A-Master', exact: true }).click({ button: 'right' })
  await page.getByRole('menuitem', { name: 'Rename master' }).click()
  await pages.getByRole('textbox', { name: 'Master name' }).fill('Body')
  await pages.getByRole('textbox', { name: 'Master name' }).press('Enter')
  await expect(pages.getByRole('option', { name: 'Body', exact: true })).toBeVisible()
})

test('a name another master has is rejected in the status bar', async ({ page }) => {
  await open(page)
  await addMaster(page)
  await addMaster(page)
  const pages = await overview(page)
  await pages.getByText('B-Master', { exact: true }).dblclick()
  await pages.getByRole('textbox', { name: 'Master name' }).fill('A-Master')
  await pages.getByRole('textbox', { name: 'Master name' }).press('Enter')
  await expect(page.getByText('a master named A-Master exists')).toBeVisible()
  await expect(pages.getByRole('option', { name: 'B-Master', exact: true })).toBeVisible()
})

test('a page draws the layers of its master under its own, which a click does not pick', async ({ page }) => {
  await applied(page)
  await expect(rects(page)).toHaveCount(2)
  await page.mouse.move(1, 1)
  const shown = await colors(page, [await inside(page), await outside(page)])
  expect(shown.map(gray)).toEqual([true, false])
  await page.mouse.click(...(await inside(page)))
  await expect(title(page)).toHaveText('Document')
})

test('ctrl+shift+click overrides a master layer with a copy on the page', async ({ page }) => {
  await overridden(page)
  await expect(rects(page)).toHaveCount(3)
  await expect(page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Reset to master' })).toBeVisible()
})

test('reset to master removes the override, undo brings it back and the page resets all its overrides', async ({ page }) => {
  await overridden(page)
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Reset to master' }).click()
  await expect(rects(page)).toHaveCount(2)
  await expect(title(page)).toHaveText('Document')
  await page.keyboard.press('Control+z')
  await expect(rects(page)).toHaveCount(3)

  await page.keyboard.press('Escape')
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Reset overrides' }).click()
  await expect(rects(page)).toHaveCount(2)
})

test('a page shows the prefix of its master as its page numbers do, in any script', async ({ page }) => {
  await open(page)
  await addMaster(page)
  const pages = await overview(page)
  await pages.getByText('A-Master', { exact: true }).dblclick()
  await pages.getByRole('textbox', { name: 'Master name' }).fill('ÄB-Master')
  await pages.getByRole('textbox', { name: 'Master name' }).press('Enter')
  await showPage(page, 1)
  await choose(page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Master' }), 'ÄB-Master')
  await expect((await overview(page)).getByTitle('Master ÄB-Master', { exact: true })).toHaveText('ÄB')
})

/** A master spread with a rectangle at the outer foot of each page, applied to pages 2 and 3. */
async function sides(page: Page) {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await addMaster(page)
  await page.keyboard.press('r')
  await drag(page, await screen(page, 10, 190, 0), await screen(page, 30, 205, 0))
  await page.keyboard.press('r')
  await drag(page, await screen(page, 118, 190), await screen(page, 138, 205))
  await expect(rects(page)).toHaveCount(2)
  for (let i = 0; i < 2; i++) {
    await addPage(page)
    await choose(panel.getByRole('combobox', { name: 'Master' }), 'A-Master')
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
  await expect(current(page, 2)).toHaveAttribute('aria-pressed', 'true')
  await expect(rects(page)).toHaveCount(1)
  await expect(panel.getByRole('region', { name: 'Layout' }).getByTitle('X in mm').getByRole('textbox')).toHaveValue('10')
  await panel.getByRole('button', { name: 'Reset to master' }).click()
  await expect(rects(page)).toHaveCount(0)
})

test('with facing pages a one-page master gets its layers on both of its pages', async ({ page }) => {
  await open(page)
  const facing = page.getByRole('complementary', { name: 'Properties' }).getByRole('combobox', { name: 'Spreads' })
  const rects = page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true })
  await choose(facing, 'Single pages')
  await addMaster(page)
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 20), await screen(page, 60, 40))
  await page.keyboard.press('Escape')
  await choose(facing, 'Facing pages')
  await expect(rects).toHaveCount(2)
  await page.mouse.move(1, 1)
  const shown = await colors(page, [await screen(page, 40, 30, 0), await screen(page, 40, 30)])
  expect(shown.map(gray)).toEqual([true, true])
  await choose(facing, 'Single pages')
  await expect(rects).toHaveCount(1)
})

test('a page number on the pages of a master spread shows the number of each page, counted on in a section', async ({ page }) => {
  await open(page)
  const panel = page.getByRole('complementary', { name: 'Properties' })
  const layers = page.getByRole('tree', { name: 'Layers' })
  await addMaster(page)
  await page.keyboard.press('t')
  await page.mouse.click(...(await screen(page, 10, 200, 0)))
  await panel.getByRole('button', { name: 'Insert character' }).click()
  await page.getByRole('menuitem', { name: 'Page number' }).click()
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await page.keyboard.press('t')
  await page.mouse.click(...(await screen(page, 130, 200)))
  await page.keyboard.press('Control+Alt+Shift+N')
  await page.keyboard.press('Escape')
  await page.keyboard.press('Escape')
  await expect(layers.getByRole('button', { name: '#', exact: true })).toHaveCount(2)
  for (let i = 0; i < 2; i++) {
    await addPage(page)
    await choose(panel.getByRole('combobox', { name: 'Master' }), 'A-Master')
  }
  await panel.getByRole('checkbox', { name: 'Start section' }).check()
  await panel.getByRole('textbox', { name: 'Section starts at' }).fill('9')
  await panel.getByRole('textbox', { name: 'Section starts at' }).press('Enter')
  await choose(panel.getByRole('combobox', { name: 'Numbering' }), 'i, ii, iii')
  await panel.getByRole('textbox', { name: 'Prefix' }).fill('p')
  await panel.getByRole('textbox', { name: 'Prefix' }).press('Enter')
  const pdf = join(mkdtempSync(join(tmpdir(), 'satz-')), 'satz.pdf')
  const download = page.waitForEvent('download')
  await (await exportButton(page)).click()
  await (await download).saveAs(pdf)
  const text = execFileSync('mutool', ['draw', '-q', '-F', 'text', '-o', '-', pdf, '2-3']).toString()
  expect(text.replace(/\s+/g, ' ').trim()).toBe('2 pix')
})
