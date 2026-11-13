import { useEffect, useRef, useState } from 'react'
import type { Editor } from './editor'
import { Keys } from './controls'
import { Icon } from './icons'
import type { Version } from './model'

const day = (t: number) => new Date(t * 1000).toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })
const time = (t: number) => new Date(t * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' })

/** Ctrl Alt Z: the versions of the document's history by day; choosing one restores it. */
export function History({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [versions] = useState(() => editor.engine.versions() as Version[])
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  const restore = (v: Version) => {
    ref.current!.close()
    if (v === versions[0]) return
    editor.apply({ type: 'restore', peer: v.peer, counter: v.counter })
    editor.say(`Restored the version of ${time(v.time)}`, { label: 'Undo', run: () => editor.apply({ type: 'undo' }) })
  }

  return (
    <dialog ref={ref} className="palette" aria-label="Version history" onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <div className="pal-top">
        <Icon name="clock" />
        <span className="pal-in">Version history</span>
        <Keys keys="Esc" />
      </div>
      <div className="pal-list" role="listbox" aria-label="Versions">
        {versions.map((v, i) => (
          <div key={`${v.peer}${v.counter}`} role="presentation">
            {(i === 0 || day(v.time) !== day(versions[i - 1].time)) && <div className="pal-g">{day(v.time)}</div>}
            <button type="button" className="pal-it" role="option" aria-selected={i === 0} onClick={() => restore(v)}>
              <span className="pal-t">{time(v.time)}</span>
              <small>{[i === 0 && 'Current', v.restored ? 'Restored' : `${v.edits} ${v.edits === 1 ? 'edit' : 'edits'}`].filter(Boolean).join(' · ')}</small>
            </button>
          </div>
        ))}
      </div>
    </dialog>
  )
}
