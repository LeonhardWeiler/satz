import { useEditor, type Editor } from './editor'
import { Icon } from './icons'

/** Picks the page of the spread whose layers are listed, and moves between spreads; on a master, fits its left or right page. */
export function Switcher({ editor }: { editor: Editor }) {
  const snapshot = useEditor(editor, (e) => e.snapshot)
  const current = useEditor(editor, (e) => e.pageId)
  const side = useEditor(editor, (e) => e.side)
  const master = snapshot.masters.find((m) => m.id === current)
  if (master) {
    const used = snapshot.pages.filter((p) => p.master === master.id).length
    return (
      <>
        {snapshot.facingPages && (
          <div className="switcher" role="group" aria-label="Master side">
            <span className="switcher-name">{master.name}</span>
            {(['left', 'right'] as const).map((s) => (
              <button key={s} type="button" aria-pressed={side === s} title={`${s === 'left' ? 'Left' : 'Right'} master page`} onClick={() => editor.set({ side: s })}>
                {s === 'left' ? 'L' : 'R'}
              </button>
            ))}
          </div>
        )}
        <div className="banner">
          <Icon name="master" />
          <span>
            Editing master <strong>{master.name}</strong>, used by {used} {used === 1 ? 'page' : 'pages'}
          </span>
          <button type="button" onClick={() => editor.exitMaster()}>
            Done<kbd>Esc</kbd>
          </button>
        </div>
      </>
    )
  }
  const { spreads, pages } = snapshot
  const k = spreads.findIndex((s) => s.includes(current))
  const go = (i: number) => editor.showPage(spreads[i][0])
  return (
    <nav className="switcher" aria-label="Spreads">
      <button type="button" aria-label="Previous spread" title="Previous spread (PgUp)" disabled={k <= 0} onClick={() => go(k - 1)}>
        <Icon name="left" />
      </button>
      {spreads[k]?.map((id) => {
        const n = pages.findIndex((p) => p.id === id) + 1
        return (
          <button key={id} type="button" aria-pressed={id === current} aria-label={`Page ${n}`} title={`Page ${n}, its layers`} onClick={() => editor.showPage(id)}>
            {n}
          </button>
        )
      })}
      <button type="button" aria-label="Next spread" title="Next spread (PgDn)" disabled={k >= spreads.length - 1} onClick={() => go(k + 1)}>
        <Icon name="chevron" />
      </button>
    </nav>
  )
}
