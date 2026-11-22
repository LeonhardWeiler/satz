import { useEffect, useRef, useState } from 'react'
import type { Editor } from './editor'
import { Keys } from './controls'
import { Icon } from './icons'
import { pageName, type Version } from './model'

const day = (t: number) => new Date(t * 1000).toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })
const time = (t: number) => new Date(t * 1000).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit', second: '2-digit' })

export function History({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [versions] = useState(() => editor.engine.versions())
  const [snapshot] = useState(editor.snapshot)
  const [was] = useState(() => ({ page: editor.pageId, selection: editor.selection }))
  const [shown, setShown] = useState(versions[0])
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  const show = (v: Version) => {
    if (v === shown) return
    setShown(v)
    editor.preview(v === versions[0] ? null : v)
  }
  const close = () => {
    ref.current!.close()
    editor.preview(null)
    onClose()
  }
  const cancel = () => {
    close()
    editor.showPage(was.page, editor.back)
    editor.set({ selection: was.selection })
  }
  const restore = (v: Version) => {
    if (v === versions[0]) return cancel()
    close()
    editor.apply({ type: 'restore', peer: v.peer, counter: v.counter })
    editor.say(`Restored the version of ${time(v.time)}`, { label: 'Undo', run: () => editor.apply({ type: 'undo' }) })
  }

  return (
    <dialog ref={ref} className="palette history" aria-label="Version history" onCancel={cancel} onClick={(e) => e.target === e.currentTarget && cancel()}>
      <div className="pal-top">
        <Icon name="clock" />
        <span className="pal-in">Version history</span>
        <Keys keys="Esc" />
      </div>
      <div className="pal-list" role="listbox" aria-label="Versions">
        {versions.map((v, i) => (
          <div key={`${v.peer}${v.counter}`} role="presentation">
            {(i === 0 || day(v.time) !== day(versions[i - 1]?.time ?? v.time)) && <div className="pal-g">{day(v.time)}</div>}
            <button type="button" className="pal-it" role="option" aria-selected={v === shown} onMouseEnter={() => show(v)} onFocus={() => show(v)} onClick={() => restore(v)}>
              <span className="pal-t">{time(v.time)}</span>
              <small>{[i === 0 && 'Current', pageName(snapshot, v.page), v.restored ? 'Restored' : `${v.edits} ${v.edits === 1 ? 'edit' : 'edits'}`].filter(Boolean).join(' · ')}</small>
            </button>
          </div>
        ))}
      </div>
    </dialog>
  )
}
