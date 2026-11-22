import { Fragment } from 'react'
import { addPage } from './commands'
import { useEditor, type Editor } from './editor'
import { Icon } from './icons'
import { pageName } from './model'
import { useSettings } from './settings'

export function Switcher({ editor }: { editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const current = useEditor(editor, (e) => e.pageId)
  const spread = useSettings().layers === 'spread'
  const master = snapshot.masters.find((m) => m.id === current)
  if (master) {
    return (
      <div className="switcher">
        <span className="switcher-total">{master.name}</span>
      </div>
    )
  }
  const { spreads, pages } = snapshot
  const k = spreads.findIndex((s) => s.includes(current))
  const show = (id?: string) => id && editor.showPage(id)
  const go = (i: number) => show(spreads[i]?.[0])
  return (
    <nav className="switcher" aria-label="Spreads">
      <button type="button" className="icon-button" aria-label="Previous spread" title="Previous spread" data-action="Previous spread" disabled={k <= 0} onClick={() => go(k - 1)}>
        <Icon name="left" />
      </button>
      <span className="switcher-at">
        {spreads[k]?.map((id, i) => {
          const name = pageName(snapshot, id)
          return (
            <Fragment key={id}>
              {i > 0 && '–'}
              <button type="button" aria-pressed={spread || id === current} aria-label={name} title={`${name}, its layers`} onClick={() => editor.showPage(id)}>
                {snapshot.labels[pages.findIndex((p) => p.id === id)]}
              </button>
            </Fragment>
          )
        })}
        <span className="switcher-total">/ {pages.length}</span>
      </span>
      <button type="button" className="icon-button" aria-label="Next spread" title="Next spread" data-action="Next spread" disabled={k >= spreads.length - 1} onClick={() => go(k + 1)}>
        <Icon name="chevron" />
      </button>
      <button type="button" className="icon-button" aria-label="Add page" title="Add page" data-action="Add page" onClick={() => addPage(editor)}>
        <Icon name="plus" />
      </button>
    </nav>
  )
}

export function MasterBanner({ editor }: { editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const current = useEditor(editor, (e) => e.pageId)
  const master = snapshot.masters.find((m) => m.id === current)
  if (!master) return null
  const used = snapshot.pages.filter((p) => p.master === master.id).length
  return (
    <div className="banner">
      <Icon name="master" />
      <span>
        Editing master <strong>{master.name}</strong>, used by {used} {used === 1 ? 'page' : 'pages'}
      </span>
      <button type="button" title="Done (Esc)" onClick={() => editor.exitMaster()}>
        Done
      </button>
    </div>
  )
}
