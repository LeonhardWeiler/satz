import { expect, it } from 'vite-plus/test'
import { decode } from './settings'

const DEFAULTS = { unit: 'mm', size: 1, keys: {} as Record<string, string>, overview: { masters: 170, pages: 200, columns: 0 }, presets: [] as string[] }

it('keeps a stored field only when its type matches the default and merges objects per field', () => {
  expect(decode(DEFAULTS, '{"unit":"pt","size":"2","overview":{"pages":300,"columns":"x"},"presets":{},"keys":{"Undo":"Ctrl+U","Redo":4},"gone":1}')).toEqual({
    ...DEFAULTS,
    unit: 'pt',
    keys: { Undo: 'Ctrl+U' },
    overview: { masters: 170, pages: 300, columns: 0 },
  })
})

it('falls back to the defaults for malformed or missing data', () => {
  for (const json of [null, '', '{', 'null', '[1]', '"x"', '{"overview":null,"keys":[]}']) expect(decode(DEFAULTS, json)).toEqual(DEFAULTS)
})
