import init, { Engine, type Inks } from './engine/engine'
import type { Snapshot } from './model'

/** What the preflight shows of the pages `pages` and of the whole document. */
export type Preview = { pages: string[]; ppi: number; on: number; limit: number; over: boolean; gamut: boolean }
export type Sheet = { page: string; width: number; height: number; image: Uint8Array; coverage: Float32Array }
export type Highest = { ink: number; page: string; layer?: string }
export type Previewed = { ppi: number; sheets: Sheet[]; spots: string[]; max: number[]; highest: Highest | null }

const engine = init().then(() => new Engine())
const inks = new Map<string, Inks>()
let stats: { ppi: number; spots: string[]; max: number[]; highest: Highest | null } | null = null

/** The inks of `page`, kept while it is asked for. */
function inksOf(e: Engine, page: string, ppi: number) {
  let i = inks.get(page)
  if (!i) {
    i = e.inks(page, ppi)!
    inks.set(page, i)
  }
  return i
}

function statsOf(e: Engine, ppi: number, keep: string[]) {
  if (stats?.ppi === ppi) return stats
  const snap: Snapshot = JSON.parse(e.snapshot())
  const max = new Map<string, number>()
  let highest: Highest | null = null
  for (const p of snap.pages) {
    const i = inksOf(e, p.id, ppi)
    const names = ['C', 'M', 'Y', 'K', ...i.spots()]
    i.max().forEach((v, k) => max.set(names[k], Math.max(v, max.get(names[k]) ?? 0)))
    const coverage = i.coverage()
    let at = 0
    for (let k = 1; k < coverage.length; k++) if (coverage[k] > coverage[at]) at = k
    if (coverage[at] > (highest?.ink ?? 0)) {
      const x = ((at % i.width) + 0.5) * (72 / ppi) - p.bleed
      const y = (Math.floor(at / i.width) + 0.5) * (72 / ppi) - p.bleed
      highest = { ink: coverage[at], page: p.id, layer: e.hit(p.id, x, y, 0).at(-1) }
    }
    if (!keep.includes(p.id)) {
      i.free()
      inks.delete(p.id)
    }
  }
  const spots = [...max.keys()].slice(4)
  stats = { ppi, spots, max: [...max.values()], highest }
  return stats
}

onmessage = async ({ data: { doc, fonts, preview } }: MessageEvent<{ doc: Uint8Array | null; fonts: Uint8Array[]; preview?: Preview }>) => {
  try {
    const e = await engine
    for (const bytes of fonts) e.addFont(bytes)
    if (doc) {
      e.load(doc)
      for (const i of inks.values()) i.free()
      inks.clear()
      stats = null
    }
    if (!preview) {
      const pdf = e.pdf()
      return postMessage({ pdf }, { transfer: [pdf.buffer] })
    }
    const { pages, ppi, on, limit, over, gamut } = preview
    const { spots, max, highest } = statsOf(e, ppi, pages)
    const sheets = pages.map((page): Sheet => {
      const i = inksOf(e, page, ppi)
      const names = i.spots()
      const mask = [0, 1, 2, 3, ...names.map((n) => 4 + spots.indexOf(n))].reduce((m, b, k) => (on & (1 << b) ? m | (1 << k) : m), 0)
      return { page, width: i.width, height: i.height, image: i.image(mask, limit, over, gamut), coverage: i.coverage() }
    })
    for (const [page, i] of inks) {
      if (pages.includes(page)) continue
      i.free()
      inks.delete(page)
    }
    const result: Previewed = { ppi, sheets, spots, max, highest }
    postMessage(result, { transfer: sheets.flatMap((s) => [s.image.buffer, s.coverage.buffer]) })
  } catch (e) {
    postMessage({ error: (e as Error).message })
  }
}
