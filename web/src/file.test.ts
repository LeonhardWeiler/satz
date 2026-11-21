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

it('export sends a linked image again once it left the document and came back', async () => {
  const sent: string[][] = []
  vi.stubGlobal('Worker', class {
    onmessage?: (e: { data: object }) => void
    postMessage(m: { images: Uint8Array[] }) {
      sent.push(m.images.map((i) => new TextDecoder().decode(i)))
      queueMicrotask(() => this.onmessage!({ data: { file: new Uint8Array() } }))
    }
  })
  let links = {}
  const editor = {
    get snapshot() { return { links, images: { a: {} } } },
    engine: { fontsAdded: () => 0, version: () => '', versionVector: () => new Uint8Array(), updates: () => new Uint8Array(), imageFile: (h: string) => new TextEncoder().encode(h) },
  } as unknown as Editor
  for (const l of [{ a: 'a.png' }, {}, { a: 'a.png' }]) {
    links = l
    await exportFile(editor, { type: 'application/pdf', pages: [0], title: '' })
  }
  expect(sent).toEqual([['a'], [], ['a']])
})
