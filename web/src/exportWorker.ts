import init, { Engine, type Inks } from './engine/engine'
import type { Snapshot } from './model'

/** What the preflight shows of the pages `pages` and of the whole document. */
export type Preview = { pages: string[]; ppi: number; on: number; limit: number; over: boolean; gamut: boolean }
/** Every page as an image of `type` at `ppi`. */
export type Image = { type: 'image/png' | 'image/jpeg'; ppi: number }
export type Sheet = { page: string; width: number; height: number; image: Uint8Array; coverage: Float32Array }
export type Previewed = { ppi: number; sheets: Sheet[]; spots: string[]; max: number[]; highest: number }

const engine = init().then(() => new Engine())
/** What a page adds to the document's stats at `ppi`, while it prints as `key`; `inks` only while it is shown. */
type Page = { key: bigint; ppi: number; inks?: Inks; names: string[]; max: number[]; ink: number }
const pages = new Map<string, Page>()

function pageOf(e: Engine, id: string, ppi: number) {
  const key = e.printed(id)!
  const old = pages.get(id)
  if (old?.key === key && old.ppi === ppi) return old
  old?.inks?.free()
  const inks = e.inks(id, ppi)!
  const ink = inks.coverage().reduce((a, b) => Math.max(a, b), 0)
  const page: Page = { key, ppi, inks, names: ['C', 'M', 'Y', 'K', ...inks.spots()], max: [...inks.max()], ink }
  pages.set(id, page)
  return page
}

onmessage = async ({ data: { doc, fonts, preview, title, image } }: MessageEvent<{ doc: Uint8Array | null; fonts: Uint8Array[]; preview?: Preview; title?: string; image?: Image }>) => {
  try {
    const e = await engine
    for (const bytes of fonts) if (bytes.length) e.addFont(bytes)
    if (doc) e.load(doc)
    if (image) {
      const snap: Snapshot = JSON.parse(e.snapshot())
      const images = await Promise.all(snap.pages.map(async ({ id }) => {
        const png = e.png(id, image.ppi)!
        if (image.type === 'image/png') return png
        const bitmap = await createImageBitmap(new Blob([png as Uint8Array<ArrayBuffer>]))
        const canvas = new OffscreenCanvas(bitmap.width, bitmap.height)
        canvas.getContext('2d')!.drawImage(bitmap, 0, 0)
        return new Uint8Array(await (await canvas.convertToBlob({ type: image.type, quality: 0.9 })).arrayBuffer())
      }))
      return postMessage({ images }, { transfer: images.map((i) => i.buffer) })
    }
    if (!preview) {
      const pdf = e.pdf(title!, new Date().toISOString().slice(0, 19) + 'Z')
      return postMessage({ pdf }, { transfer: [pdf.buffer] })
    }
    const { pages: shown, ppi, on, limit, over, gamut } = preview
    const snap: Snapshot = JSON.parse(e.snapshot())
    const max = new Map<string, number>()
    let highest = 0
    for (const { id } of snap.pages) {
      const p = pageOf(e, id, ppi)
      p.names.forEach((name, k) => max.set(name, Math.max(p.max[k], max.get(name) ?? 0)))
      highest = Math.max(highest, p.ink)
    }
    for (const [id, p] of pages) {
      if (shown.includes(id)) continue
      p.inks?.free()
      p.inks = undefined
      if (!snap.pages.some((q) => q.id === id)) pages.delete(id)
    }
    const spots = [...max.keys()].slice(4)
    const sheets = shown.map((id): Sheet => {
      const p = pages.get(id)!
      const i = (p.inks ??= e.inks(id, ppi)!)
      const mask = [0, 1, 2, 3, ...p.names.slice(4).map((n) => 4 + spots.indexOf(n))].reduce((m, b, k) => (on & (1 << b) ? m | (1 << k) : m), 0)
      return { page: id, width: i.width, height: i.height, image: i.image(mask, limit, over, gamut), coverage: i.coverage() }
    })
    const result: Previewed = { ppi, sheets, spots, max: [...max.values()], highest }
    postMessage(result, { transfer: sheets.flatMap((s) => [s.image.buffer, s.coverage.buffer]) })
  } catch (e) {
    postMessage({ error: (e as Error).message })
  }
}
