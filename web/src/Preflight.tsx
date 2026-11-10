import { useEffect, useState, useSyncExternalStore } from 'react'
import { press } from './commands'
import { Field, Section, Select } from './controls'
import { useEditor, type Editor } from './editor'
import { length, useSettings } from './settings'
import { preview } from './file'
import { Icon } from './icons'
import type { Issue, Preset } from './model'

const PPI = 48
const PLATES = [['Cyan', 'c'], ['Magenta', 'm'], ['Yellow', 'y'], ['Black', 'k']]

const problem = (i: Issue, editor: Editor) =>
  i.problem === 'missingFont' ? `Missing font ${i.font}`
  : i.problem === 'overset' ? `Overset text · ${count(words(editor, i.layer), 'word')} left`
  : i.problem === 'shortOfBleed' ? 'Short of the bleed'
  : i.problem === 'lowPpi' ? `Image at ${Math.round(i.ppi)} ppi`
  : i.problem === 'gamut' ? 'Color outside the gamut'
  : i.problem === 'ink' ? `${Math.round(i.ink)} % ink`
  : i.problem === 'nearTrim' ? `${length(i.distance)} from the trim`
  : i.problem === 'missingGlyph' ? `The font lacks ${JSON.stringify(i.character)}`
  : 'RGB color'

export const isError = (i: Issue) => ['overset', 'missingFont', 'missingGlyph', 'shortOfBleed', 'ink'].includes(i.problem)
const options = (values: number[], unit: string, zero: string, v: number) =>
  Object.fromEntries([...new Set([0, ...values, v])].map((n) => [String(n), n ? `${n} ${unit}` : zero]))
const count = (n: number, what: string) => `${n} ${what}${n === 1 ? '' : 's'}`
const words = (editor: Editor, id: string) => {
  const n = editor.nodes.get(id)?.node
  return n?.kind === 'text' ? editor.oversetWords(n) : 0
}

/** Keeps the inks of the shown pages from the worker in `editor.previewed` while mounted, between drags. */
function usePreview(editor: Editor, on: boolean) {
  useEffect(() => {
    if (!on) return
    let [key, busy, again, gone] = ['', false, false, false]
    const request = () => {
      const pages = editor.spread.map((p) => p.id).filter((id) => editor.snapshot.pages.some((p) => p.id === id))
      const next = JSON.stringify([editor.engine.version(), pages, editor.inks])
      if (next === key || gone || editor.groups) return
      if (busy) return void (again = true)
      ;[key, busy] = [next, true]
      preview(editor, { pages, ppi: PPI, ...editor.inks, limit: editor.snapshot.inkLimit })
        .then(
          (previewed) => gone || editor.set({ previewed }),
          (e: Error) => editor.fail(`Could not show the inks: ${e.message}`),
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
  }, [editor, on])
}

const PRESETS: Record<Preset, string> = { x4: 'PDF/X-4, print', x1a: 'PDF/X-1a, print', screen: 'PDF, screen' }

/** The indices of the pages that `text` names by the numbers they show or their place, all for an empty text, null when one is not there. */
function pagesOf(text: string, labels: string[]) {
  if (!text.trim()) return [...labels.keys()]
  const at = (s: string) => {
    const i = labels.findIndex((l) => l.toLowerCase() === s.trim().toLowerCase())
    return i >= 0 ? i : /^\s*\d+\s*$/.test(s) && +s >= 1 && +s <= labels.length ? +s - 1 : -1
  }
  const span = (part: string): [number, number] | null => {
    if (at(part) >= 0) return [at(part), at(part)]
    for (const m of part.matchAll(/[-–]/g)) {
      const [a, b] = [at(part.slice(0, m.index)), at(part.slice(m.index + 1))]
      if (a >= 0 && b >= a) return [a, b]
    }
    return null
  }
  const out = new Set<number>()
  for (const part of text.split(',')) {
    const r = span(part)
    if (!r) return null
    for (let i = r[0]; i <= r[1]; i++) out.add(i)
  }
  return [...out].sort((x, y) => x - y)
}

const FORMATS = { pdf: 'PDF', png: 'PNG', jpeg: 'JPEG', zip: 'Package' } as const
export type Format = keyof typeof FORMATS

export function Preflight({ editor, exporting, onExport }: { editor: Editor; exporting: boolean; onExport: (format: Format, pages: number[]) => void }) {
  const [format, setFormat] = useState<Format>('pdf')
  const [range, setRange] = useState('')
  const issues = useEditor(editor, (e) => e.snapshot.preflight)
  const pages = useEditor(editor, (e) => e.snapshot.pages)
  const masters = useEditor(editor, (e) => e.snapshot.masters)
  const profile = useEditor(editor, (e) => e.snapshot.profile) ?? 'FOGRA51'
  const selection = useEditor(editor, (e) => e.selection)
  useSettings()
  const inks = useEditor(editor, (e) => e.inks)
  const limit = useEditor(editor, (e) => e.snapshot.inkLimit)
  const { preset, cropMarks, includeBleed, colorMode, imagePpi, jpegQuality, rasterPpi } = useEditor(editor, (e) => e.snapshot)
  const previewed = useEditor(editor, (e) => e.previewed)
  const swatches = useEditor(editor, (e) => e.snapshot.swatches)
  const pointerInk = useSyncExternalStore(editor.subscribePointer, () => editor.pointerInk)
  usePreview(editor, colorMode === 'cmyk')

  const labels = useEditor(editor, (e) => e.snapshot.labels)
  const chosen = pagesOf(range, labels)
  const sorted = [...issues].sort((a, b) => Number(isError(b)) - Number(isError(a)))
  const errors = issues.filter(isError).length
  const where = (id: string) => {
    const n = pages.findIndex((p) => p.id === id)
    return n < 0 ? masters.find((m) => m.id === id)?.name : `Page ${labels[n]}`
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
      <span className="plate-max">{previewed ? `max ${Math.round(previewed.max[bit] ?? 0)} %` : '–'}</span>
    </label>
  )
  const highest = previewed?.highest ?? 0

  return (
    <section className="preflight" aria-label="Preflight">
      <header className="pf-head">
        <h2>Preflight</h2>
        <span className="pf-sum">{issues.length > 0 && `${count(errors, 'error')}, ${count(issues.length - errors, 'warning')}`}</span>
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
                  <span className="iss-m" title={`${problem(i, editor)} · ${i.name}`}>
                    <strong>{problem(i, editor)}</strong>
                    <small>{i.name}</small>
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
        {colorMode === 'cmyk' && (
          <>
          <Section title="Proof">
            <p className="pf-note">Simulates print, {profile}</p>
            <label className="check">
              <input type="checkbox" checked={inks.gamut} onChange={(e) => set({ gamut: e.currentTarget.checked })} />
              Mark colors outside the gamut
            </label>
          </Section>
          <Section title="Separations">
            <div className="plates">
              {PLATES.map(([name, cls], bit) => plate(bit, name, cls))}
              {(previewed?.spots ?? swatches.filter((s) => s.spot).map((s) => s.name)).map((name, i) => plate(4 + i, name, 'spot'))}
            </div>
          </Section>
          <Section title="Ink coverage">
            <div className="grid">
              <Field label="Limit" title="Ink limit" unit="%" int min={200} max={400} reset={300} value={limit} onCommit={(inkLimit) => editor.apply({ type: 'setDocument', inkLimit })} />
              <label className="check">
                <input type="checkbox" checked={inks.over} onChange={(e) => set({ over: e.currentTarget.checked })} />
                Mark above limit
              </label>
            </div>
            <div className="kv">
              <span>Highest</span>
              <strong className={highest > limit + 0.5 ? 'bad' : ''}>
                {previewed ? `${Math.round(highest)} %` : '–'}
              </strong>
            </div>
            <div className="kv">
              <span>Under pointer</span>
              <strong title={pointerInk === null ? 'Point at a color' : undefined}>{pointerInk === null ? '—' : `${pointerInk} %`}</strong>
            </div>
          </Section>
          </>
        )}
        <Section title="Export">
          <Select label="Format" prefix="Format" value={format} options={FORMATS} onChange={setFormat} />
          {(format === 'pdf' || format === 'zip') && (
            <>
              <Select
                label="Preset"
                prefix="Preset"
                value={preset}
                options={colorMode === 'rgb' ? { ...PRESETS, x1a: 'PDF/X-1a, print, CMYK documents only' } : PRESETS}
                disabled={colorMode === 'rgb' ? ['x1a'] : []}
                onChange={(preset) => editor.apply({ type: 'setDocument', preset })}
              />
              <label className="check">
                <input type="checkbox" checked={cropMarks && preset !== 'screen'} disabled={preset === 'screen'} onChange={(e) => editor.apply({ type: 'setDocument', cropMarks: e.currentTarget.checked })} />
                Crop marks
              </label>
              <label className="check">
                <input type="checkbox" checked={includeBleed && preset !== 'screen'} disabled={preset === 'screen'} onChange={(e) => editor.apply({ type: 'setDocument', includeBleed: e.currentTarget.checked })} />
                Include {length(pages[0]?.bleed ?? 0)} bleed
              </label>
              <Select label="Image resolution" prefix="Images" value={String(imagePpi)} options={options([72, 150, 300], 'ppi', 'Keep', imagePpi)} onChange={(v) => editor.apply({ type: 'setDocument', imagePpi: Number(v) })} />
              <Select label="JPEG quality" prefix="JPEG quality" value={String(jpegQuality)} options={options([90, 75], '%', 'Lossless', jpegQuality)} onChange={(v) => editor.apply({ type: 'setDocument', jpegQuality: Number(v) })} />
            </>
          )}
          <label className="field">
            <span className="field-label">Pages</span>
            <input
              name="pages"
              aria-label="Pages to export"
              aria-invalid={chosen === null}
              autoComplete="off"
              placeholder="All"
              value={range}
              onChange={(e) => setRange(e.currentTarget.value)}
            />
          </label>
          {chosen === null && <p className="pf-error">{labels.length > 1 ? `Pages ${labels[0]}–${labels.at(-1)}, e.g. 1-3, 5` : `Only page ${labels[0]}`}</p>}
          {(format === 'png' || format === 'jpeg') && (
            <Field label="Resolution" title="Resolution" unit="ppi" min={72} max={1200} reset={300} value={rasterPpi} onCommit={(rasterPpi) => editor.apply({ type: 'setDocument', rasterPpi })} />
          )}
          <button type="button" className={errors ? 'button pf-go bad' : 'primary pf-go'} disabled={exporting || chosen === null} onClick={() => chosen && onExport(format, chosen)}>
            {!exporting && errors > 0 && <Icon name="warn" />}
            {exporting ? 'Exporting…' : errors ? `Export with ${count(errors, 'error')}` : `Export ${FORMATS[format]}`}
          </button>
          {exporting && (
            <div className="pf-bar">
              <i />
            </div>
          )}
        </Section>
      </div>
    </section>
  )
}
