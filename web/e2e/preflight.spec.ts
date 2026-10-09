import { expect, test, addPage, choose, colors, current, drag, exportButton, near, open, openExample, pixels, preflight, screen } from './util'

test('preflight of an rgb document leaves out the inks and lists a layer short of the bleed and a click selects it on its page with the quick edit bar', async ({ page }) => {
  await open(page)
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle', exact: true }).last().click()
  await page.getByRole('complementary', { name: 'Properties' }).getByRole('button', { name: 'Fill color' }).click()
  await page.getByRole('dialog', { name: 'Fill color' }).getByRole('textbox', { name: 'Hex' }).fill('00ff00')
  await page.getByRole('dialog', { name: 'Fill color' }).getByRole('textbox', { name: 'Hex' }).press('Enter')
  await page.keyboard.press('Escape')
  const pf = await preflight(page)
  await expect(pf.getByRole('region', { name: 'Separations' })).toHaveCount(0)
  const [green] = await colors(page, [await screen(page, 74, 7)])
  expect(near(green, [0, 255, 0])).toBe(true)
  const short = pf.getByRole('button', { name: /Short of the bleed/ })
  await expect(short).toHaveCount(0)

  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, -1), await screen(page, 40, 20))
  await expect(short).toHaveCount(1)
  await addPage(page)

  await short.click()
  await expect(current(page, 1)).toHaveAttribute('aria-pressed', 'true')
  await expect(short).toHaveAttribute('aria-current', 'true')
  const layers = page.getByRole('tree', { name: 'Layers' })
  await expect(layers.getByRole('treeitem', { selected: true })).toHaveCount(1)
  await expect(page.getByRole('toolbar', { name: 'Quick edit' })).toBeVisible()
})

test('export names the errors of the preflight but still downloads the pdf', async ({ page }) => {
  await open(page)
  const exportPdf = await exportButton(page)
  await expect(exportPdf).toHaveText('Export PDF')
  let download = page.waitForEvent('download')
  await exportPdf.click()
  await download
  await expect(page.getByText('Exported Untitled.pdf')).toBeVisible({ timeout: 30000 })
  const box = (await page.getByText('Exported Untitled.pdf').boundingBox())!
  const go = (await exportPdf.boundingBox())!
  expect(box.y + box.height).toBeLessThan(go.y)
  expect(Math.abs(box.x + box.width - go.x - go.width)).toBeLessThan(2)

  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, -1), await screen(page, 40, 20))
  await expect(exportPdf).toHaveText('Export with 1 error')
  download = page.waitForEvent('download')
  await exportPdf.click()
  await download
})

test('export writes every page or a range as a png or a jpeg, and a package as a zip', async ({ page }) => {
  await open(page)
  await addPage(page)
  const go = await exportButton(page)
  const region = page.getByRole('region', { name: 'Preflight' })
  await choose(region.getByRole('combobox', { name: 'Format' }), 'PNG')
  await expect(region.getByRole('combobox', { name: 'Preset' })).toHaveCount(0)
  await expect(go).toHaveText('Export PNG')
  const names: string[] = []
  const bytes: Buffer[] = []
  page.on('download', async (d) => {
    names.push(d.suggestedFilename())
    bytes.push(Buffer.from(await (await d.createReadStream()).toArray().then((c) => Buffer.concat(c))))
  })
  await go.click()
  await expect(page.getByText('Exported 2 pages')).toBeVisible({ timeout: 30000 })
  await expect.poll(() => bytes.length).toBe(2)
  expect(names).toEqual(['Untitled-1.png', 'Untitled-2.png'])
  expect(bytes[0].subarray(1, 4).toString()).toBe('PNG')

  await choose(region.getByRole('combobox', { name: 'Format' }), 'JPEG')
  const range = region.getByRole('textbox', { name: 'Pages to export' })
  await range.fill('3')
  await expect(go).toBeDisabled()
  await range.fill('2-2')
  await go.click()
  await expect.poll(() => bytes.length).toBe(3)
  expect(names[2]).toBe('Untitled-2.jpg')
  expect([...bytes[2].subarray(0, 2)]).toEqual([0xff, 0xd8])

  await choose(region.getByRole('combobox', { name: 'Format' }), 'Package')
  await expect(region.getByRole('combobox', { name: 'Preset' })).toBeVisible()
  await expect(go).toHaveText('Export Package')
  await go.click()
  await expect.poll(() => bytes.length).toBe(4)
  expect(names[3]).toBe('Untitled.zip')
  expect(bytes[3].subarray(0, 4).toString('latin1')).toBe('PK\x03\x04')
  expect(bytes[3].includes('Untitled/Untitled.pdf')).toBe(true)
})

test('preflight opens from its shortcut, the counter and export, and esc ends it', async ({ page }) => {
  await open(page)
  const region = await preflight(page)
  await expect(region).toBeVisible()
  await expect.poll(async () => (await page.locator('.right').boundingBox())?.width).toBe(284)
  await page.keyboard.press('Escape')
  await expect(region).toHaveCount(0)

  await page.getByRole('button', { name: /^Preflight, \d+ issues?$/ }).click()
  await expect(region).toBeVisible()
  await page.keyboard.press('Escape')

  await page.keyboard.press('Control+Shift+E')
  await expect(region.getByRole('button', { name: 'Export PDF' })).toBeFocused()
})

test('preflight in the booklet separates the plates, reports ink over the limit and zooms to an issue', async ({ page }) => {
  await openExample(page, 'booklet.satz')
  const region = await preflight(page)
  const issues = region.locator('.iss')
  await expect(issues).toHaveCount(0)

  await expect(region.getByText(/^max \d+ %$/).first()).toBeVisible({ timeout: 30000 })
  const box = (await page.getByLabel('Page canvas').boundingBox())!
  const area = [box.x, box.y, box.width, box.height] as const
  const before = await pixels(page, ...area)
  await region.getByRole('checkbox', { name: 'Black' }).uncheck()
  await expect.poll(async () => JSON.stringify(await pixels(page, ...area)) !== JSON.stringify(before)).toBe(true)
  await region.getByRole('checkbox', { name: 'Black' }).check()

  const limit = region.getByRole('textbox', { name: 'Ink limit in %' })
  await limit.fill('500')
  await limit.press('Enter')
  await expect(limit).toHaveValue('400')
  await limit.fill('200')
  await limit.press('Enter')
  await expect(region.getByRole('button', { name: /% ink/ }).first()).toBeVisible()
  await expect(region.locator('.kv strong.bad')).toHaveText(/^\d+ %/)

  const zoom = await page.getByLabel('Zoom').textContent()
  await issues.first().click()
  await expect(page.getByLabel('Zoom')).not.toHaveText(zoom!)
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('treeitem', { selected: true })).toHaveCount(1)

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await expect(region.getByText(/^\d+ %$/).last()).toBeVisible()
})

test('the preflight of a cmyk booklet never scrolls sideways', async ({ page }) => {
  await page.setViewportSize({ width: 1448, height: 1160 })
  await page.goto('')
  await page.getByRole('dialog', { name: 'New document' }).getByRole('button', { name: /Satz booklet/ }).click()
  await expect(page).toHaveTitle('booklet.satz - Satz')
  const region = await preflight(page)
  await expect(region.getByText(/^max \d+ %$/).first()).toBeVisible({ timeout: 30000 })
  const body = region.locator('.pf-body')
  expect(await body.evaluate((b) => b.scrollWidth - b.clientWidth)).toBe(0)
})
