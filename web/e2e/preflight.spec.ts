import { expect, test } from '@playwright/test'
import { addPage, current, drag, open, screen } from './util'

test('preflight lists a layer short of the bleed and a click selects it on its page', async ({ page }) => {
  await open(page)
  const preflight = page.getByRole('region', { name: 'Preflight' })
  const short = preflight.getByRole('button', { name: /Short of the bleed/ })
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
})

test('export warns of preflight issues but still downloads the pdf', async ({ page }) => {
  await open(page)
  const warning = page.getByText(/preflight issue/)
  const exportPdf = page.getByRole('button', { name: 'Export', exact: true })

  let download = page.waitForEvent('download')
  await exportPdf.click()
  await download
  await expect(warning).toHaveCount(0)

  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, -1), await screen(page, 40, 20))
  download = page.waitForEvent('download')
  await exportPdf.click()
  await download
  await expect(warning).toHaveText('Exported with 1 preflight issue. See Preflight.')
})
