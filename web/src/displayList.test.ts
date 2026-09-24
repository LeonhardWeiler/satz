import { readFileSync } from 'node:fs'
import { expect, test } from 'vitest'
import { close, decode, type Op } from './displayList'

const read = (name: string) => readFileSync(new URL(`testdata/${name}`, import.meta.url))

test('decodes what the rust encoder wrote', () => {
  const bytes = read('display-list.bin')
  const words = new Uint32Array(bytes.buffer, bytes.byteOffset, bytes.length / 4)
  const plain = JSON.parse(
    JSON.stringify(decode(words), (_, v) => (ArrayBuffer.isView(v) ? Array.from(v as Float32Array) : v)),
  )
  expect(plain).toMatchObject(JSON.parse(read('display-list.json').toString()))
})

test('close finds the matching end, with endMask between beginMask and popMask', () => {
  const item: Op = { op: 'beginItem', item: 0, hash: 0 }
  const ops: Op[] = [{ op: 'beginMask' }, item, { op: 'endItem' }, { op: 'endMask' }, item, { op: 'endItem' }, { op: 'popMask' }]
  expect(close(ops, 0)).toBe(3)
  expect(close(ops, 3)).toBe(6)
  expect(close(ops, 1)).toBe(2)
})
