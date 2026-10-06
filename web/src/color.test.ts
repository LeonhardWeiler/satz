import { expect, it } from 'vitest'
import { cmykText, parseCmyk } from './color'

it('reads cmyk in the usual notations and writes one of them', () => {
  const want = [0.2, 0.4, 0, 0.1]
  for (const t of ['20, 40, 0, 10', '20 40 0 10', '20 / 40 / 0 / 10', 'C20 M40 Y0 K10', 'C:20 M:40 Y:0 K:10', '20% 40% 0% 10%', ' c: 20 m: 40 y: 0 k: 10\n'])
    expect(parseCmyk(t)).toEqual(want)
  for (const t of ['20 40 0', '20 40 0 10 5', 'M20 C40 Y0 K10', '20 40 0 110', '#ff0000', 'C20M40Y0K10']) expect(parseCmyk(t)).toBeNull()
  expect(cmykText(want)).toBe('C:20 M:40 Y:0 K:10')
  expect(parseCmyk(cmykText([0.125, 1, 0, 0.5]))).toEqual([0.125, 1, 0, 0.5])
})
