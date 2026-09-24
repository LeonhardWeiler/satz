import { expect, test } from 'vitest'
import { penPath } from './pen'

const MOVE = 0
const LINE = 1
const CUBIC = 4
const CLOSE = 5

test('corner anchors are joined by lines', () => {
  const path = penPath([{ x: 0, y: 0, hx: 0, hy: 0 }, { x: 10, y: 0, hx: 0, hy: 0 }], false)
  expect(path).toEqual([MOVE, 0, 0, LINE, 10, 0])
})

test('a dragged anchor gets mirrored handles', () => {
  const path = penPath([{ x: 0, y: 0, hx: 0, hy: 0 }, { x: 10, y: 0, hx: 0, hy: 5 }, { x: 20, y: 0, hx: 0, hy: 0 }], false)
  expect(path).toEqual([MOVE, 0, 0, CUBIC, 0, 0, 10, -5, 10, 0, CUBIC, 10, 5, 20, 0, 20, 0])
})

test('a closed path returns to the first anchor', () => {
  const path = penPath([{ x: 0, y: 0, hx: 0, hy: 0 }, { x: 10, y: 0, hx: 0, hy: 0 }, { x: 0, y: 10, hx: 0, hy: 0 }], true)
  expect(path).toEqual([MOVE, 0, 0, LINE, 10, 0, LINE, 0, 10, LINE, 0, 0, CLOSE])
})
