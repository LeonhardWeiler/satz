import { readFileSync } from 'node:fs'
import { expect, test } from 'vitest'
import { decode } from './displayList'

const read = (name: string) => readFileSync(new URL(`testdata/${name}`, import.meta.url))

test('decodes what the rust encoder wrote', () => {
  const bytes = read('display-list.bin')
  const words = new Uint32Array(bytes.buffer, bytes.byteOffset, bytes.length / 4)
  const plain = JSON.parse(
    JSON.stringify(decode(words), (_, v) => (ArrayBuffer.isView(v) ? Array.from(v as Float32Array) : v)),
  )
  expect(plain).toEqual(JSON.parse(read('display-list.json').toString()))
})
