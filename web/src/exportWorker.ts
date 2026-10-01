import init, { Engine, type Inks } from './engine/engine'
import type { Snapshot } from './model'

/** What the preflight shows of the pages `pages` and of the whole document. */
export type Preview = { pages: string[]; ppi: number; on: number; limit: number; over: boolean; gamut: boolean }
export type Sheet = { page: string; width: number; height: number; image: Uint8Array; coverage: Float32Array }
export type Highest = { ink: number; page: string; layer?: string }
export type Previewed = { ppi: number; sheets: Sheet[]; spots: string[]; max: number[]; highest: Highest | null }

const engine = init().then(() => new Engine())
/** What a page adds to the document's stats at `ppi`, while it prints as `key`; `inks` only while it is shown. */
type Page = { key: bigint; ppi: number; inks?: Inks; names: string[]; max: number[]; ink: number; layer?: string }
const pages = new Map<string, Page>()

function pageOf(e: Engine, id: string, bleed: number, ppi: number) {
  const key = e.printed(id)!
  const old = pages.get(id)
  if (old?.key === key && old.ppi === ppi) return old
  old?.inks?.free()
  const inks = e.inks(id, ppi)!
  const coverage = inks.coverage()
  let at = 0
  for (let k = 1; k < coverage.length; k++) if (coverage[k] > coverage[at]) at = k
  const x = ((at % inks.width) + 0.5) * (72 / ppi) - bleed
  const y = (Math.floor(at / inks.width) + 0.5) * (72 / ppi) - bleed
  const page: Page = { key, ppi, inks, names: ['C', 'M', 'Y', 'K', ...inks.spots()], max: [...inks.max()], ink: coverage[at], layer: e.hit(id, x, y, 0).at(-1) }
  pages.set(id, page)
  return page
}

onmessage = async ({ data: { doc, fonts, preview, title } }: MessageEvent<{ doc: Uint8Array | null; fonts: Uint8Array[]; preview?: Preview; title?: string }>) => {
  try {
    const e = await engine
    for (const bytes of fonts) if (bytes.length) e.addFont(bytes)
    if (doc) e.load(doc)
    if (!preview) {
      const pdf = e.pdf(title!, new Date().toISOString().slice(0, 19) + 'Z')
      return postMessage({ pdf }, { transfer: [pdf.buffer] })
    }
    const { pages: shown, ppi, on, limit, over, gamut } = preview
    const snap: Snapshot = JSON.parse(e.snapshot())
    const max = new Map<string, number>()
    let highest: Highest | null = null
    for (const { id, bleed } of snap.pages) {
      const p = pageOf(e, id, bleed, ppi)
      p.names.forEach((name, k) => max.set(name, Math.max(p.max[k], max.get(name) ?? 0)))
      if (p.ink > (highest?.ink ?? 0)) highest = { ink: p.ink, page: id, layer: p.layer }
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
