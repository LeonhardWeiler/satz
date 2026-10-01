import { readFile } from 'node:fs/promises'
import { type FileChooser, type Page } from '@playwright/test'
import { expect, test, addPage, autosaved, drawn, open, pageCount, screen } from './util'


/** Saves the document as Firefox does, by a download, and returns the file. */
async function save(page: Page, name = 'Untitled.satz') {
  const download = page.waitForEvent('download')
  await page.keyboard.press('Control+s')
  const file = await download
  expect(file.suggestedFilename()).toBe(name)
  return { name: file.suggestedFilename(), mimeType: 'application/x-satz', buffer: await readFile(await file.path()) }
}

async function choose(page: Page, files: Parameters<FileChooser['setFiles']>[0]) {
  page.once('dialog', (d) => d.accept())
  const chooser = page.waitForEvent('filechooser')
  await page.keyboard.press('Control+o')
  await (await chooser).setFiles(files)
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as Partial<Record<'showSaveFilePicker' | 'showOpenFilePicker', unknown>>
    delete w.showSaveFilePicker
    delete w.showOpenFilePicker
  })
  await open(page)
  await expect(page).toHaveTitle('Untitled.satz - Satz')
})

test('a reload keeps the document and its unsaved mark', async ({ page }) => {
  await addPage(page)
  await expect(page).toHaveTitle('* Untitled.satz - Satz')
  await autosaved(page)
  await page.reload()
  await pageCount(page, 2)
  await expect(page).toHaveTitle('* Untitled.satz - Satz')
})

test('a reloaded document has nothing to undo', async ({ page }) => {
  await addPage(page)
  await autosaved(page)
  await page.reload()
  await pageCount(page, 2)
  await page.keyboard.press('Control+z')
  await drawn(page)
  await pageCount(page, 2)
})

test('saving downloads the file and clears the unsaved mark', async ({ page }) => {
  await addPage(page)
  await save(page)
  await expect(page).toHaveTitle('Untitled.satz - Satz')
})

test('a double click beside the name renames the document but not its extension for the next save', async ({ page }) => {
  const box = (await page.getByRole('heading', { name: 'Untitled.satz' }).boundingBox())!
  await page.mouse.dblclick(box.x + box.width + 20, box.y + box.height / 2)
  const input = page.getByRole('textbox', { name: 'Document name' })
  await expect(input).toHaveValue('Untitled')
  await input.fill('Report')
  await page.keyboard.press('Enter')
  await expect(page.getByRole('heading', { name: 'Report.satz' })).toBeVisible()
  await expect(page).toHaveTitle('* Report.satz - Satz')
  await save(page, 'Report.satz')
  await expect(page).toHaveTitle('Report.satz - Satz')
})

test('opening a file replaces the document with it', async ({ page }) => {
  await addPage(page)
  const file = await save(page)
  await addPage(page)
  await choose(page, file)
  await expect(page).toHaveTitle('Untitled.satz - Satz')
  await pageCount(page, 2)
})

test('a file that is not a Satz document is not opened and says why', async ({ page }) => {
  await addPage(page)
  await choose(page, { name: 'notes.satz', mimeType: 'application/octet-stream', buffer: Buffer.from('notes') })
  await expect(
    page.getByText('Could not open notes.satz: not a Satz document. Choose a .satz file saved by Satz.'),
  ).toBeVisible()
  await pageCount(page, 2)
})

test('a dropped document opens and a dropped image is placed with a click', async ({ page }) => {
  await addPage(page)
  const file = await save(page)
  await addPage(page)
  const image = await readFile(new URL('../../examples/earthrise.jpg', import.meta.url))
  const dropFile = async (name: string, bytes: Buffer) => {
    await page.evaluate(
      ([name, bytes]) => {
        const dataTransfer = new DataTransfer()
        dataTransfer.items.add(new File([new Uint8Array(bytes)], name))
        document.querySelector('.canvas')!.dispatchEvent(new DragEvent('drop', { dataTransfer, bubbles: true, cancelable: true }))
      },
      [name, [...bytes]] as const,
    )
  }
  page.once('dialog', (d) => d.accept())
  await dropFile(file.name, file.buffer)
  await expect(page).toHaveTitle('Untitled.satz - Satz')
  await pageCount(page, 2)
  await dropFile('earthrise.jpg', image)
  const [x, y] = await screen(page, 74, 105)
  await page.mouse.click(x, y)
  await expect(page.getByRole('tree', { name: 'Layers' }).getByRole('button', { name: 'earthrise.jpg' })).toBeVisible()
})

test('new asks before it discards changes and then starts over', async ({ page }) => {
  await addPage(page)
  const sample = page.getByRole('dialog', { name: 'New document' }).getByRole('button', { name: 'Sample' })
  page.once('dialog', (d) => d.dismiss())
  await page.keyboard.press('Control+Alt+n')
  await sample.click()
  await page.keyboard.press('Escape')
  await drawn(page)
  await pageCount(page, 2)
  page.once('dialog', (d) => d.accept())
  await page.keyboard.press('Control+Alt+n')
  await sample.click()
  await pageCount(page, 1)
  await expect(page).toHaveTitle('Untitled.satz - Satz')
})
