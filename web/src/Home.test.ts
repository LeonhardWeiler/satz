import { expect, test, vi } from 'vite-plus/test'
import { ago } from './Home'

test('ago rounds edit times down', () => {
  vi.useFakeTimers({ now: 1e12 })
  const at = (min: number) => ago(1e12 - min * 60000)
  expect([at(0.5), at(1.9), at(59.9), at(90), at(1410)]).toEqual(['Edited just now', 'Edited 1 min ago', 'Edited 59 min ago', 'Edited 1 h ago', 'Edited 23 h ago'])
  vi.useRealTimers()
})
