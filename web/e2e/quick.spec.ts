import { expect, test, drag, open, screen } from './util'

test('quick edit sits above the selection, sets radius and fill, and hides while dragging, editing and in preflight', async ({ page }) => {
  await open(page)
  const quick = page.getByRole('toolbar', { name: 'Quick edit' })
  const panel = page.getByRole('complementary', { name: 'Properties' })
  await expect(quick).toBeHidden()

  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 20), await screen(page, 60, 60))
  await expect(quick).toBeVisible()
  const box = (await quick.boundingBox())!
  expect(box.y + box.height).toBeLessThan((await screen(page, 20, 20))[1])

  const radius = quick.getByRole('textbox', { name: 'Corner radius in mm' })
  await radius.fill('4')
  await radius.press('Enter')
  await expect(panel.getByRole('textbox', { name: 'Corner radius in mm' }).first()).toHaveValue('4')

  const stroke = page.getByRole('menu', { name: 'Stroke color' })
  await expect(quick.getByRole('textbox', { name: 'Stroke weight' })).toHaveCount(0)
  await quick.getByTitle('Stroke color').click()
  await stroke.getByRole('menuitemradio', { name: 'Black' }).click()
  const weight = quick.getByRole('textbox', { name: 'Stroke weight' })
  await weight.fill('2')
  await weight.press('Enter')
  await expect(panel.getByRole('textbox', { name: 'Stroke weight' })).toHaveValue('2')

  const fill = page.getByRole('menu', { name: 'Fill color' })
  await page.getByRole('button', { name: 'Add swatch' }).click()
  await page.keyboard.press('Escape')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle' }).first().click()
  await quick.getByTitle('Fill color').click()
  await fill.locator('[role^=menuitem]').last().click()
  await expect(fill).toHaveCount(0)
  await quick.getByTitle('Fill color').click()
  await fill.getByRole('menuitemradio', { name: 'None' }).click()
  await expect(panel.getByRole('region', { name: 'Fill' }).locator('.paint')).toHaveCount(0)

  await page.mouse.move(...(await screen(page, 40, 40)))
  await page.mouse.down()
  await page.mouse.move(...(await screen(page, 50, 50)), { steps: 4 })
  await expect(quick).toBeHidden()
  await page.mouse.up()
  await expect(quick).toBeVisible()

  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle' }).first().click()
  await expect(quick).toBeVisible()
  await page.keyboard.press('Control+Alt+Y')
  await expect(quick).toBeHidden()
  await page.keyboard.press('Escape')

  await page.keyboard.press('t')
  await drag(page, await screen(page, 20, 100), await screen(page, 80, 130))
  await expect(quick).toBeHidden()
})

test('the circles inside a rectangle round its corners, with ctrl only the one dragged', async ({ page }) => {
  await open(page)
  const radius = page.getByRole('complementary', { name: 'Properties' }).getByRole('textbox', { name: 'Corner radius in mm' })
  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, 20), await screen(page, 60, 60))
  const [x, y] = await screen(page, 20, 20)
  const mm = (await screen(page, 21, 20))[0] - x
  await drag(page, [x + 12, y + 12], [x + 12 + 5 * mm, y + 12 + 5 * mm])
  await expect(radius).toHaveValue('5')
  const [r, b] = await screen(page, 60, 60)
  await page.keyboard.down('Control')
  await drag(page, [r - 5 * mm, b - 5 * mm], [r - 8 * mm, b - 8 * mm])
  await page.keyboard.up('Control')
  await expect(radius).toHaveValue('Mixed')
  await expect(page.getByRole('toolbar', { name: 'Quick edit' }).getByRole('textbox', { name: 'Corner radius in mm' })).toHaveValue('Mixed')
  await radius.fill('2')
  await radius.press('Enter')
  await expect(radius).toHaveValue('2')
})

test('an ellipse opens to an arc and hollows to a ring', async ({ page }) => {
  await open(page)
  const props = page.getByRole('complementary', { name: 'Properties' })
  await page.keyboard.press('o')
  await drag(page, await screen(page, 20, 20), await screen(page, 60, 60))
  for (const [name, value] of [['Arc sweep in %', '50'], ['Inner radius in %', '40'], ['Arc start in °', '-90']]) {
    const field = props.getByRole('textbox', { name })
    await field.fill(value)
    await field.press('Enter')
  }
  await expect(props.getByRole('textbox', { name: 'Arc sweep in %' })).toHaveValue('50')
  await expect(props.getByRole('textbox', { name: 'Inner radius in %' })).toHaveValue('40')
  await expect(props.getByRole('textbox', { name: 'Arc start in °' })).toHaveValue('270')
})
