import { expect, test, drag, open, screen } from './util'

test('quick edit sits above the selection, sets radius, opacity and fill, and hides while dragging, editing and in preflight', async ({ page }) => {
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
  const opacity = quick.getByRole('textbox', { name: 'Opacity' })
  await opacity.fill('50')
  await opacity.press('Enter')
  await expect(panel.getByRole('textbox', { name: 'Opacity' }).first()).toHaveValue('50')

  const swatches = page.getByRole('menu', { name: 'Swatches' })
  await quick.getByTitle('Fill swatch').click()
  await expect(swatches.getByRole('menuitem', { name: 'No swatches' })).toBeDisabled()
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Add swatch' }).click()
  await page.keyboard.press('Escape')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle' }).first().click()
  await quick.getByTitle('Fill swatch').click()
  await swatches.locator('[role^=menuitem]').first().click()
  await expect(swatches).toHaveCount(0)
  await expect(quick.getByTitle('Fill swatch')).toBeVisible()

  await page.mouse.move(...(await screen(page, 40, 40)))
  await page.mouse.down()
  await page.mouse.move(...(await screen(page, 50, 50)), { steps: 4 })
  await expect(quick).toBeHidden()
  await page.mouse.up()
  await expect(quick).toBeVisible()

  await quick.getByRole('button', { name: 'More' }).click()
  await expect(panel.locator(':focus')).toHaveCount(1)

  await page.keyboard.press('Escape')
  await page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'Rectangle' }).first().click()
  await expect(quick).toBeVisible()
  await page.keyboard.press('Control+Alt+Y')
  await expect(quick).toBeHidden()
  await page.keyboard.press('Escape')

  await page.keyboard.press('t')
  await drag(page, await screen(page, 20, 100), await screen(page, 80, 130))
  await expect(quick).toBeHidden()
})
