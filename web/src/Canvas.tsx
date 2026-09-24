import { useEffect, useRef, useState } from 'react'
import type { CanvasKit, Surface } from 'canvaskit-wasm'
import type { Engine } from './engine/engine'
import type { Snapshot } from './model'
import { Renderer, fitView, type View } from './renderer'

const PX_PER_PT = 96 / 72

export function Canvas({ ck, engine }: { ck: CanvasKit; engine: Engine }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const [zoom, setZoom] = useState(0)

  useEffect(() => {
    const canvas = ref.current!
    const renderer = new Renderer(ck, engine)
    const page = (engine.snapshot() as Snapshot).pages[0]
    const view: View = { x: 0, y: 0, zoom: 1 }
    let surface: Surface | null = null
    let frame = 0
    let fitted = false
    let space = false
    let drag: { x: number; y: number } | null = null

    const redraw = () => {
      if (frame) return
      frame = requestAnimationFrame(() => {
        frame = 0
        if (!surface) return
        renderer.draw(surface.getCanvas(), view, canvas.width / canvas.clientWidth)
        surface.flush()
      })
    }
    const zoomAt = (px: number, py: number, zoom: number) => {
      zoom = Math.min(256, Math.max(0.02, zoom))
      view.x = px - ((px - view.x) * zoom) / view.zoom
      view.y = py - ((py - view.y) * zoom) / view.zoom
      view.zoom = zoom
      setZoom(zoom)
      redraw()
    }
    const fit = () => {
      Object.assign(view, fitView(page, canvas.clientWidth, canvas.clientHeight))
      setZoom(view.zoom)
      redraw()
    }

    const resize = new ResizeObserver(([entry]) => {
      const box = entry.devicePixelContentBoxSize?.[0]
      canvas.width = box ? box.inlineSize : Math.round(entry.contentRect.width * devicePixelRatio)
      canvas.height = box ? box.blockSize : Math.round(entry.contentRect.height * devicePixelRatio)
      surface?.delete()
      surface = ck.MakeWebGLCanvasSurface(canvas)
      if (!fitted) {
        fitted = true
        fit()
      }
      redraw()
    })
    try {
      resize.observe(canvas, { box: 'device-pixel-content-box' })
    } catch {
      resize.observe(canvas)
    }

    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      const scale = e.deltaMode === 1 ? 16 : 1
      if (e.ctrlKey || e.metaKey) {
        zoomAt(e.offsetX, e.offsetY, view.zoom * Math.exp(-e.deltaY * scale * 0.01))
      } else {
        view.x -= (e.shiftKey && !e.deltaX ? e.deltaY : e.deltaX) * scale
        view.y -= (e.shiftKey && !e.deltaX ? 0 : e.deltaY) * scale
        redraw()
      }
    }
    const onPointerDown = (e: PointerEvent) => {
      if (e.button !== 1 && !(space && e.button === 0)) return
      e.preventDefault()
      canvas.setPointerCapture(e.pointerId)
      drag = { x: e.clientX, y: e.clientY }
      canvas.dataset.panning = ''
    }
    const onPointerMove = (e: PointerEvent) => {
      if (!drag) return
      view.x += e.clientX - drag.x
      view.y += e.clientY - drag.y
      drag = { x: e.clientX, y: e.clientY }
      redraw()
    }
    const onPointerUp = () => {
      drag = null
      delete canvas.dataset.panning
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.code === 'Space') {
        space = e.type === 'keydown'
        canvas.toggleAttribute('data-space', space)
        return
      }
      if (e.type !== 'keydown') return
      const cx = canvas.clientWidth / 2
      const cy = canvas.clientHeight / 2
      const mod = e.ctrlKey || e.metaKey
      if (e.shiftKey && e.code === 'Digit1') fit()
      else if (e.shiftKey && e.code === 'Digit0') zoomAt(cx, cy, PX_PER_PT)
      else if (mod && (e.key === '=' || e.key === '+')) zoomAt(cx, cy, view.zoom * 2)
      else if (mod && e.key === '-') zoomAt(cx, cy, view.zoom / 2)
      else return
      e.preventDefault()
    }

    canvas.addEventListener('wheel', onWheel, { passive: false })
    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    canvas.addEventListener('pointercancel', onPointerUp)
    window.addEventListener('keydown', onKey)
    window.addEventListener('keyup', onKey)
    return () => {
      cancelAnimationFrame(frame)
      resize.disconnect()
      canvas.removeEventListener('wheel', onWheel)
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.removeEventListener('pointercancel', onPointerUp)
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('keyup', onKey)
      renderer.delete()
      surface?.delete()
    }
  }, [ck, engine])

  return (
    <>
      <canvas ref={ref} className="canvas" aria-label="Page canvas" />
      <output className="zoom" aria-label="Zoom">
        {Math.round((zoom / PX_PER_PT) * 100)}%
      </output>
    </>
  )
}
