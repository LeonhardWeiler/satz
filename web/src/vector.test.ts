import { expect, test } from 'vitest'
import { contours, nearest, remove, shift, split, toPath } from './vector'

const [MOVE, LINE, CUBIC, CLOSE] = [0, 1, 4, 5]
const square = [MOVE, 0, 0, LINE, 10, 0, LINE, 10, 10, LINE, 0, 10, LINE, 0, 0, CLOSE]
const arc = [MOVE, 0, 0, CUBIC, 0, 5, 5, 10, 10, 10]

test('a path parses into knots and back, a closed one without its closing knot', () => {
  const cs = contours(square)
  expect(cs).toHaveLength(1)
  expect(cs[0].closed).toBe(true)
  expect(cs[0].knots.map((k) => [k.x, k.y])).toEqual([[0, 0], [10, 0], [10, 10], [0, 10]])
  expect(toPath(cs)).toEqual(square)
  expect(toPath(contours(arc))).toEqual(arc)
})

test('a knot splits a line where it is nearest and keeps it a line', () => {
  const cs = contours(square)
  const near = nearest(cs, { x: 5, y: 1 })
  expect(near.at).toEqual([0, 0])
  expect(near.t).toBeCloseTo(0.5)
  expect(toPath(split(cs, near.at, near.t)).slice(0, 9)).toEqual([MOVE, 0, 0, LINE, 5, 0, LINE, 10, 0])
  expect(nearest(cs, { x: 1, y: 5 }).at).toEqual([0, 3])
})

test('a knot splits a curve into two curves along it', () => {
  const [c] = split(contours(arc), [0, 0], 0.5)
  expect(c.knots).toHaveLength(3)
  expect(c.knots[1]).toMatchObject({ x: 3.125, y: 6.875 })
  expect(toPath([c])[3]).toBe(CUBIC)
})

test('a removed knot joins its neighbours and a contour of one knot goes', () => {
  expect(toPath(remove(contours(square), [[0, 1]]))).toEqual([MOVE, 0, 0, LINE, 10, 10, LINE, 0, 10, LINE, 0, 0, CLOSE])
  expect(remove(contours([MOVE, 0, 0, LINE, 1, 1]), [[0, 0]])).toEqual([])
})

test('a moved knot takes its handles along, a moved handle only itself', () => {
  const cs = contours(arc)
  expect(shift(cs, [[0, 1]], 'point', 1, 2)[0].knots[1]).toMatchObject({ x: 11, y: 12, ix: 6, iy: 12 })
  expect(shift(cs, [[0, 0]], 'out', 1, 0)[0].knots[0]).toMatchObject({ x: 0, y: 0, ox: 1, oy: 5 })
})
