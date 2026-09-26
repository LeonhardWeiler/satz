import { readFile } from 'node:fs/promises'
import { expect, test, type FileChooser, type Page } from '@playwright/test'
import { autosaved, drawn, open } from './util'

const pages = (page: Page) => page.getByRole('navigation', { name: 'Pages' })
const row = (page: Page, n: number) => pages(page).getByRole('button', { name: `Page ${n}`, exact: true })

async function addPage(page: Page, n: number) {
  await pages(page).getByRole('button', { name: 'Add page' }).click()
  await expect(row(page, n)).toBeVisible()
}

/** Saves the document as Firefox does, by a download, and returns the file. */
async function save(page: Page) {
  const download = page.waitForEvent('download')
  await page.keyboard.press('Control+s')
  const file = await download
  expect(file.suggestedFilename()).toBe('Untitled.satz')
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
  await expect(page).toHaveTitle('Untitled.satz — Satz')
})

test('a reload keeps the document and its unsaved mark', async ({ page }) => {
  await addPage(page, 2)
  await expect(page).toHaveTitle('* Untitled.satz — Satz')
  await autosaved(page)
  await page.reload()
  await expect(row(page, 2)).toBeVisible()
  await expect(page).toHaveTitle('* Untitled.satz — Satz')
})

test('a reloaded document has nothing to undo', async ({ page }) => {
  await addPage(page, 2)
  await autosaved(page)
  await page.reload()
  await expect(row(page, 2)).toBeVisible()
  await page.keyboard.press('Control+z')
  await drawn(page)
  await expect(row(page, 2)).toBeVisible()
})

test('saving downloads the file and clears the unsaved mark', async ({ page }) => {
  await addPage(page, 2)
  await save(page)
  await expect(page).toHaveTitle('Untitled.satz — Satz')
})

test('opening a file replaces the document with it', async ({ page }) => {
  await addPage(page, 2)
  const file = await save(page)
  await addPage(page, 3)
  await choose(page, file)
  await expect(page).toHaveTitle('Untitled.satz — Satz')
  await expect(pages(page).getByRole('button', { name: /^Page \d$/ })).toHaveCount(2)
})

test('a file that is not a Satz document is not opened and says why', async ({ page }) => {
  await addPage(page, 2)
  await choose(page, { name: 'notes.satz', mimeType: 'application/octet-stream', buffer: Buffer.from('notes') })
  await expect(
    page.getByText('Could not open notes.satz: not a Satz document. Choose a .satz file saved by Satz.'),
  ).toBeVisible()
  await expect(row(page, 2)).toBeVisible()
})

test('new asks before it discards changes and then starts over', async ({ page }) => {
  await addPage(page, 2)
  page.once('dialog', (d) => d.dismiss())
  await page.keyboard.press('Control+Alt+n')
  await drawn(page)
  await expect(row(page, 2)).toBeVisible()
  page.once('dialog', (d) => d.accept())
  await page.keyboard.press('Control+Alt+n')
  await expect(pages(page).getByRole('button', { name: /^Page \d$/ })).toHaveCount(1)
  await expect(page).toHaveTitle('Untitled.satz — Satz')
})
