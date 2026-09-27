import { describe, expect, it } from 'vitest'
import { evalExpr, step } from './field'

describe('evalExpr', () => {
  it('reads numbers with a comma or a point', () => {
    expect(evalExpr('12,5', 0, 'mm')).toBe(12.5)
    expect(evalExpr(' .5 ', 0, 'mm')).toBe(0.5)
  })

  it('computes with precedence and brackets', () => {
    expect(evalExpr('2+3*4', 0, 'mm')).toBe(14)
    expect(evalExpr('(2+3)×4', 0, 'mm')).toBe(20)
    expect(evalExpr('10 − 2 x 3', 0, 'mm')).toBe(4)
    expect(evalExpr('-(4-6)', 0, 'mm')).toBe(2)
  })

  it('applies a leading operator to the current value', () => {
    expect(evalExpr('+5', 10, 'mm')).toBe(15)
    expect(evalExpr('*2', 10, 'mm')).toBe(20)
    expect(evalExpr('/4', 10, 'mm')).toBe(2.5)
    expect(evalExpr('-5', 10, 'mm')).toBe(-5)
  })

  it('converts units into the unit of the field', () => {
    expect(evalExpr('1cm', 0, 'mm')).toBe(10)
    expect(evalExpr('1in', 0, 'mm')).toBeCloseTo(25.4)
    expect(evalExpr('72pt', 0, 'mm')).toBeCloseTo(25.4)
    expect(evalExpr('1in', 0, 'pt')).toBeCloseTo(72)
    expect(evalExpr('10mm + 1cm', 0, 'mm')).toBe(20)
    expect(evalExpr('50%', 0, '%')).toBe(50)
    expect(evalExpr('3mm', 0, '°')).toBe(3)
  })

  it('rejects what is not an expression', () => {
    for (const s of ['', 'abc', '1+', '(1', '1)', '2..3', '1/0', '5km']) expect(evalExpr(s, 1, 'mm')).toBeNull()
  })
})

describe('step', () => {
  it('is 1, 10 with Shift and 0.1 with Alt except on integer fields', () => {
    expect(step({ shiftKey: false, altKey: false })).toBe(1)
    expect(step({ shiftKey: true, altKey: false })).toBe(10)
    expect(step({ shiftKey: false, altKey: true })).toBe(0.1)
    expect(step({ shiftKey: false, altKey: true }, true)).toBe(1)
  })
})
