import { useEffect } from 'react'
import { press } from './commands'
import { Field, Section, Select } from './controls'
import { MM, useEditor, type Editor } from './editor'
import { preview } from './file'
import { Icon } from './icons'
import type { Issue, Preset } from './model'

const PPI = 48
const PLATES = [['Cyan', 'c'], ['Magenta', 'm'], ['Yellow', 'y'], ['Black', 'k']]

const problem = (i: Issue) =>
  i.problem === 'missingFont' ? `Missing font ${i.font}`
  : i.problem === 'overset' ? 'Overset text'
  : i.problem === 'shortOfBleed' ? 'Short of the bleed'
  : i.problem === 'lowPpi' ? `Image at ${Math.round(i.ppi)} ppi`
  : i.problem === 'gamut' ? 'Colour outside the gamut'
  : i.problem === 'ink' ? `Ink at ${Math.round(i.ink)} %, above the limit`
  : i.problem === 'nearTrim' ? `${Math.round((i.distance / MM) * 10) / 10} mm from the trim`
  : 'RGB color'

export const isError = (i: Issue) => ['overset', 'missingFont', 'shortOfBleed', 'ink'].includes(i.problem)
const count = (n: number, what: string) => `${n} ${what}${n === 1 ? '' : 's'}`

/** Keeps the inks of the shown pages from the worker in `editor.previewed` while mounted. */
function usePreview(editor: Editor) {
  useEffect(() => {
    let [key, busy, again, gone] = ['', false, false, false]
    const request = () => {
      const pages = editor.spread.map((p) => p.id).filter((id) => editor.snapshot.pages.some((p) => p.id === id))
      const next = JSON.stringify([editor.engine.version(), pages, editor.inks])
      if (next === key || gone) return
      if (busy) return void (again = true)
      ;[key, busy] = [next, true]
      preview(editor, { pages, ppi: PPI, ...editor.inks, limit: editor.snapshot.inkLimit })
        .then(
          (previewed) => gone || editor.set({ previewed }),
          (e: Error) => editor.say(`Could not show the inks: ${e.message}`),
        )
        .finally(() => {
          busy = false
          if (again) {
            again = false
            request()
          }
        })
    }
    request()
    const off = editor.subscribe(request)
    return () => {
      gone = true
      off()
      editor.set({ previewed: null })
    }
  }, [editor])
}

const PRESETS: Record<Preset, string> = { x4: 'PDF/X-4, print', x1a: 'PDF/X-1a, print', screen: 'PDF, screen' }

export function Preflight({ editor, exporting, onExport }: { editor: Editor; exporting: boolean; onExport: () => void }) {
  const issues = useEditor(editor, (e) => e.snapshot.preflight)
  const pages = useEditor(editor, (e) => e.snapshot.pages)
  const masters = useEditor(editor, (e) => e.snapshot.masters)
  const selection = useEditor(editor, (e) => e.selection)
  const inks = useEditor(editor, (e) => e.inks)
  const limit = useEditor(editor, (e) => e.snapshot.inkLimit)
  const { preset, cropMarks, includeBleed, colorMode } = useEditor(editor, (e) => e.snapshot)
  const previewed = useEditor(editor, (e) => e.previewed)
  const pointerInk = useEditor(editor, (e) => e.pointerInk)
  usePreview(editor)

  const sorted = [...issues].sort((a, b) => Number(isError(b)) - Number(isError(a)))
  const errors = issues.filter(isError).length
  const where = (id: string) => {
    const n = pages.findIndex((p) => p.id === id)
    return n < 0 ? masters.find((m) => m.id === id)?.name : `Page ${n + 1}`
  }
  const show = (i: Issue) => {
    editor.showPage(i.page)
    editor.set({ selection: [i.layer] })
    press('Shift 2')
  }
  const set = (patch: Partial<Editor['inks']>) => editor.set({ inks: { ...inks, ...patch } })
  const plate = (bit: number, name: string, cls: string) => (
    <label key={name} className={`plate plate-${cls}`}>
      <input type="checkbox" checked={(inks.on & (1 << bit)) !== 0} onChange={() => set({ on: inks.on ^ (1 << bit) })} />
      <i className="ink" />
      <span>{name}</span>
      <span className="plate-max">{previewed ? `max ${Math.round(previewed.max[bit] ?? 0)} %` : ''}</span>
    </label>
  )
  const highest = previewed?.highest
  const highName = highest?.layer && editor.lookup(highest.layer)?.node.name

  return (
    <section className="preflight" aria-label="Preflight">
      <header className="pf-head">
        <h2>Preflight</h2>
        <span className="pf-sum">
          {issues.length ? `${count(errors, 'error')}, ${count(issues.length - errors, 'warning')}` : 'Ready to print'}
        </span>
        <button type="button" className="icon-button" aria-label="Close preflight (Esc)" title="Close preflight (Esc)" onClick={() => editor.togglePreflight()}>
          <Icon name="close" />
        </button>
      </header>
      <div className="pf-body">
        <Section title="Issues">
          {sorted.length ? (
            <div className="pf-list">
              {sorted.map((i) => (
                <button
                  key={`${i.layer} ${i.problem}`}
                  type="button"
                  className={`iss ${isError(i) ? 'error' : 'warn'}`}
                  aria-current={selection.includes(i.layer)}
                  onClick={() => show(i)}
                >
                  <Icon name={isError(i) ? 'error' : 'warn'} />
                  <span className="iss-m">
                    <strong>{i.name}</strong> {problem(i)}
                  </span>
                  <span className="iss-w">{where(i.page)}</span>
                </button>
              ))}
            </div>
          ) : (
            <p className="pf-empty">
              <Icon name="ok" />
              No issues. The document is ready for print.
            </p>
          )}
        </Section>
        <Section title="Proof">
          <p className="pf-note">Simulates print, FOGRA51</p>
          <label className="check">
            <input type="checkbox" checked={inks.gamut} onChange={(e) => set({ gamut: e.currentTarget.checked })} />
            Mark colours outside the gamut
          </label>
        </Section>
        <Section title="Separations">
          <div className="plates">
            {PLATES.map(([name, cls], bit) => plate(bit, name, cls))}
            {previewed?.spots.map((name, i) => plate(4 + i, name, 'spot'))}
          </div>
        </Section>
        <Section title="Ink coverage">
          <div className="grid">
            <Field label="Limit" title="Ink limit in %" unit="%" int min={200} max={400} value={limit} onCommit={(inkLimit) => editor.apply({ type: 'setDocument', inkLimit })} />
            <label className="check">
              <input type="checkbox" checked={inks.over} onChange={(e) => set({ over: e.currentTarget.checked })} />
              Mark above limit
            </label>
          </div>
          <div className="kv">
            <span>Highest</span>
            <strong className={highest && highest.ink > limit + 0.5 ? 'bad' : ''}>
              {highest ? `${Math.round(highest.ink)} %` : '0 %'}
              {highest && <span className="plate-max"> {[highName, where(highest.page)?.toLowerCase()].filter(Boolean).join(', ')}</span>}
            </strong>
          </div>
          <div className="kv">
            <span>Under pointer</span>
            <strong>{pointerInk === null ? 'Point at a colour' : `${pointerInk} %`}</strong>
          </div>
        </Section>
        <Section title="Export">
          <Select
            label="Preset"
            value={preset}
            options={colorMode === 'rgb' ? { ...PRESETS, x1a: 'PDF/X-1a, print, CMYK documents only' } : PRESETS}
            disabled={colorMode === 'rgb' ? ['x1a'] : []}
            onChange={(preset) => editor.apply({ type: 'setDocument', preset })}
          />
          <label className="check">
            <input type="checkbox" checked={cropMarks} disabled={preset === 'screen'} onChange={(e) => editor.apply({ type: 'setDocument', cropMarks: e.currentTarget.checked })} />
            Crop marks
          </label>
          <label className="check">
            <input type="checkbox" checked={includeBleed} disabled={preset === 'screen'} onChange={(e) => editor.apply({ type: 'setDocument', includeBleed: e.currentTarget.checked })} />
            Include {Math.round((pages[0]?.bleed ?? 0) / MM)} mm bleed
          </label>
          <button type="button" className="primary pf-go" disabled={exporting} onClick={onExport}>
            {exporting ? 'Exporting…' : errors ? `Export with ${count(errors, 'error')}` : 'Export PDF'}
          </button>
          <div className={`pf-bar${exporting ? ' run' : ''}`}>
            <i />
          </div>
        </Section>
      </div>
    </section>
  )
}
