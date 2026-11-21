import { expect, it } from 'vite-plus/test'
import { once } from './file'

it('once caches a result and retries after a rejection', async () => {
  let calls = 0
  const load = once(() => (++calls === 1 ? Promise.reject(new Error('offline')) : Promise.resolve(calls)))
  await expect(load()).rejects.toThrow('offline')
  expect(await load()).toBe(2)
  expect(await load()).toBe(2)
  expect(calls).toBe(2)
})
