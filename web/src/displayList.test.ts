import { readFileSync } from 'node:fs'
import { expect, test } from 'vite-plus/test'
import { close, decode, link, type Op } from './displayList'

const read = (name: string) => readFileSync(new URL(`testdata/${name}`, import.meta.url))

test('decodes what the rust encoder wrote', () => {
  const bytes = read('display-list.bin')
  const words = new Uint32Array(bytes.buffer, bytes.byteOffset, bytes.length / 4)
  const plain = JSON.parse(
    JSON.stringify(decode(words), (_, v: unknown) => (ArrayBuffer.isView(v) ? Array.from(v as Float32Array) : v)),
  ) as unknown
  expect(plain).toMatchObject(JSON.parse(read('display-list.json').toString()))
})

test('link sets the matching end, with endMask between beginMask and popMask', () => {
  const item = (): Op => ({ op: 'beginItem', item: 0, hash: 0 })
  const ops = link([{ op: 'beginMask' }, item(), { op: 'endItem' }, { op: 'endMask' }, item(), { op: 'endItem' }, { op: 'popMask' }])
  expect([0, 3, 1, 4].map((i) => close(ops, i))).toEqual([3, 6, 2, 5])
})

test('a list cut short throws instead of reading past its end', () => {
  const bytes = read('display-list.bin')
  const words = new Uint32Array(bytes.buffer, bytes.byteOffset, bytes.length / 4)
  for (const cut of [5, 7, 30]) expect(() => decode(words.slice(0, words.length - cut))).toThrow('display list: ends at word')
})
