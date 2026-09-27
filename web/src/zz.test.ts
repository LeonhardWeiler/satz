import { readFileSync } from 'node:fs'
import { test } from 'vitest'
import { Engine, initSync } from './engine/engine'
import type { Snapshot } from './model'

const median = (runs: number, f: () => void) => {
  const t: number[] = []
  for (let i = 0; i < runs; i++) {
    const s = performance.now()
    f()
    t.push(performance.now() - s)
  }
  return t.sort((a, b) => a - b)[runs >> 1].toFixed(1)
}
const out = (...a: unknown[]) => process.stderr.write(a.join(' ') + '\n')
const para = 'Satz sets type in frames that thread from page to page, with styles, spans and a baseline grid for body text.'

test('measure', () => {
  initSync({ module: readFileSync(new URL('engine/engine_bg.wasm', import.meta.url)) })
  out('pages snapshot displayList hit setFrame+snapshot keystroke pdf')
  for (const n of [1, 8, 16, 32]) {
    const engine = new Engine()
    const snap = (): Snapshot => JSON.parse(engine.snapshot())
    let id = snap().pages[0].id
    while (snap().pages.length < n) id = engine.apply({ type: 'duplicatePage', id })[0]
    const s = snap()
    const frames = s.pages.slice(0, 16).map((p) => p.children.find((c) => c.kind === 'text')!.id)
    for (const f of frames.slice(1)) engine.apply({ type: 'setText', id: f, text: '' })
    engine.apply({ type: 'setText', id: frames[0], text: Array(4 * frames.length).fill(para).join('\n') })
    for (let i = 1; i < frames.length; i++) engine.apply({ type: 'thread', from: frames[i - 1], to: frames[i] })
    const page = s.pages[0].id
    const rect = s.pages[0].children[0].id
    let x = 0
    const at = 4 * para.length * frames.length / 2
    out(
      n,
      median(21, () => engine.snapshot()),
      median(21, () => engine.displayList(page)),
      median(21, () => engine.hit(page, 60, 100, 1)),
      median(21, () => {
        x = 10 - x
        engine.apply({ type: 'setFrame', id: rect, x, y: 0, w: 100, h: 100 })
        JSON.parse(engine.snapshot())
      }),
      median(21, () => {
        engine.apply({ type: 'editText', id: frames[0], range: [at, at], text: 'X' })
        JSON.parse(engine.snapshot())
        engine.displayList(page)
        engine.hit(page, 60, 100, 1)
      }),
      median(3, () => engine.pdf()),
    )
    engine.free()
  }
})
