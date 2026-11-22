import { Keys } from './controls'
import { keysFor } from './keys'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'

const DELAY = 500
const MARGIN = 8

export function Tooltip() {
  const [tip, setTip] = useState<{ text: string; keys: string; at: DOMRect } | null>(null)
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    let el: HTMLElement | null = null
    let timer = 0
    let shown = 0
    const hide = () => {
      clearTimeout(timer)
      if (el?.dataset.tip !== undefined) {
        el.title = el.dataset.tip
        delete el.dataset.tip
      }
      el = null
      setTip((t) => {
        if (t) shown = performance.now()
        return null
      })
    }
    const over = (e: PointerEvent) => {
      const t = (e.target as Element).closest?.<HTMLElement>('[title], [data-tip]') ?? null
      if (t === el) return
      hide()
      if (!t?.title || e.pointerType === 'touch') return
      const tip = t.title
      el = t
      t.dataset.tip = tip
      t.removeAttribute('title')
      const show = () => {
        const m = /^(.*) \(((?:Ctrl|Alt|Shift|Space|Enter|Esc|Del|[A-Z0-9]|[^\w\s])(?: .*)?)\)$/.exec(tip)
        const [text, keys] = t.dataset.action ? [tip, keysFor(t.dataset.action)] : m ? [m[1] ?? tip, m[2] ?? ''] : [tip, '']
        setTip({ text, keys, at: t.getBoundingClientRect() })
      }
      if (performance.now() - shown < DELAY) show()
      else timer = window.setTimeout(show, DELAY)
    }
    const cancel = () => {
      hide()
      shown = 0
    }
    document.addEventListener('pointerover', over)
    document.addEventListener('pointerdown', cancel, true)
    document.addEventListener('keydown', cancel, true)
    document.addEventListener('wheel', cancel, true)
    return () => {
      hide()
      document.removeEventListener('pointerover', over)
      document.removeEventListener('pointerdown', cancel, true)
      document.removeEventListener('keydown', cancel, true)
      document.removeEventListener('wheel', cancel, true)
    }
  }, [])
  useLayoutEffect(() => {
    const el = ref.current
    if (!tip || !el) return
    el.showPopover()
    const { offsetWidth: w, offsetHeight: h } = el
    const above = tip.at.top - 6 - h >= MARGIN
    el.style.left = `${Math.max(MARGIN, Math.min(tip.at.left + tip.at.width / 2 - w / 2, window.innerWidth - MARGIN - w))}px`
    el.style.top = `${above ? tip.at.top - 6 - h : tip.at.bottom + 6}px`
  }, [tip])
  if (!tip) return null
  return (
    <div ref={ref} className="tooltip" role="tooltip" popover="manual">
      {tip.text}
      {tip.keys && <Keys keys={tip.keys} />}
    </div>
  )
}
