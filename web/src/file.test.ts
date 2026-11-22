import { expect, it, vi } from 'vite-plus/test'
import type { Editor } from './editor'
import { exportFile, once } from './file'

it('once caches a result and retries after a rejection', async () => {
  let calls = 0
  const load = once(() => (++calls === 1 ? Promise.reject(new Error('offline')) : Promise.resolve(calls)))
  await expect(load()).rejects.toThrow('offline')
  expect(await load()).toBe(2)
  expect(await load()).toBe(2)
  expect(calls).toBe(2)
})

it('export sends added fonts once and a linked image again once it left the document and came back', async () => {
  const sent: [number[], string[]][] = []
  vi.stubGlobal('Worker', class {
    onmessage?: (e: { data: object }) => void
    postMessage(m: { fonts: Uint8Array[]; images: Uint8Array[] }) {
      sent.push([m.fonts.map((f) => f[0]!), m.images.map((i) => new TextDecoder().decode(i))])
      queueMicrotask(() => this.onmessage!({ data: { file: new Uint8Array() } }))
    }
  })
  let links = {}
  const editor = {
    get snapshot() { return { links, images: { a: {} } } },
    engine: { fontsAdded: () => 5, fontsBundled: () => 4, font: (i: number) => Uint8Array.of(i), version: () => '', versionVector: () => new Uint8Array(), updates: () => new Uint8Array(), imageFile: (h: string) => new TextEncoder().encode(h) },
  } as unknown as Editor
  for (const l of [{ a: 'a.png' }, {}, { a: 'a.png' }]) {
    links = l
    await exportFile(editor, { type: 'application/pdf', pages: [0], title: '' })
  }
  expect(sent).toEqual([[[4], ['a']], [[], []], [[], ['a']]])
})
