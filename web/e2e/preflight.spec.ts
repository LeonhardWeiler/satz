import { expect, test } from '@playwright/test'
import { drag, open, screen } from './util'

test('preflight lists a layer short of the bleed and a click selects it on its page', async ({ page }) => {
  await open(page)
  const preflight = page.getByRole('region', { name: 'Preflight' })
  const pages = page.getByRole('navigation', { name: 'Pages' })
  const short = preflight.getByRole('button', { name: /Short of the bleed/ })
  await expect(short).toHaveCount(0)

  await page.keyboard.press('r')
  await drag(page, await screen(page, 20, -1), await screen(page, 40, 20))
  await expect(short).toHaveCount(1)
  await pages.getByRole('button', { name: 'Add page' }).click()

  await short.click()
  await expect(pages.getByRole('button', { name: 'Page 1', exact: true })).toHaveAttribute('aria-current', 'page')
  await expect(short).toHaveAttribute('aria-current', 'true')
  const layers = page.getByRole('tree', { name: 'Layers' })
  await expect(layers.getByRole('treeitem', { selected: true })).toHaveCount(1)
})

test('export warns of preflight issues but still downloads the pdf', async ({ page }) => {
  await open(page)
  const warning = page.getByText(/preflight issue/)
  const exportPdf = page.getByRole('button', { name: 'Export PDF' })

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
