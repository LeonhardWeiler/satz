const UNIT: Record<string, number> = { mm: 1, cm: 10, pt: 25.4 / 72, in: 25.4 }

export function evalExpr(src: string, cur: number, unit: string): number | null {
  let s = src.trim().replace(/,/g, '.').replace(/[×x]/g, '*').replace(/−/g, '-').replace(/\s+/g, '')
  if (/^[+*/]/.test(s)) s = cur + s
  const toks = s.match(/\d*\.?\d+(?:mm|cm|pt|in|%)?|[-+*/()]/g) ?? []
  if (!s || toks.join('') !== s) return null
  let i = 0
  const expr = (): number => {
    let v = term()
    while (toks[i] === '+' || toks[i] === '-') v = toks[i++] === '+' ? v + term() : v - term()
    return v
  }
  const term = (): number => {
    let v = fac()
    while (toks[i] === '*' || toks[i] === '/') v = toks[i++] === '*' ? v * fac() : v / fac()
    return v
  }
  const fac = (): number => {
    const t = toks[i++]
    if (t === '-') return -fac()
    if (t === '(') {
      const v = expr()
      return toks[i++] === ')' ? v : NaN
    }
    const m = /^(\d*\.?\d+)(mm|cm|pt|in|%)?$/.exec(t ?? '')
    if (!m) return NaN
    return m[2] && UNIT[m[2]] && UNIT[unit] ? (+m[1] * UNIT[m[2]]) / UNIT[unit] : +m[1]
  }
  const v = expr()
  return i === toks.length && Number.isFinite(v) ? v : null
}

export const step = (e: { shiftKey: boolean; altKey: boolean }, int?: boolean) => (e.shiftKey ? 10 : e.altKey && !int ? 0.1 : 1)
