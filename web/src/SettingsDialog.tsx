import { useEffect, useRef, useState, type ReactNode } from 'react'
import { Select } from './controls'
import { Icon, type IconName } from './icons'
import { setSettings, useSettings, type Unit } from './settings'
import { Shortcuts } from './Shortcuts'

const UNITS: Record<Unit, string> = { mm: 'Millimeters', cm: 'Centimeters', in: 'Inches', pt: 'Points' }
const PAGES = { general: ['General', 'settings'], screen: ['Screen size', 'fit'], shortcuts: ['Keyboard shortcuts', 'help'] } satisfies Record<string, [string, IconName]>
export type SettingsPage = keyof typeof PAGES

/** Things of a known size to hold against the screen: name, how, width and height in mm, corner radius in mm. */
const REFERENCES: Record<string, [string, string, number, number?, number?]> = {
  card: ['Bank card', 'Hold a bank or ID card against the outline', 85.6, 53.98, 3.18],
  cm: ['1 cm', 'Hold a ruler against the bar and match 1 cm', 10],
  ruler: ['10 cm', 'Hold a ruler against the bar and match 10 cm', 100],
  a4: ['A4 width', 'Hold the short side of an A4 sheet against the bar', 210],
  letter: ['US Letter width', 'Hold the short side of a US Letter sheet against the bar', 215.9],
}

function Row({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <div className="settings-row">
      <div>
        {title}
        {note && <small>{note}</small>}
      </div>
      {children}
    </div>
  )
}

/** Ctrl+,: the settings of this browser, with the keyboard shortcuts on their own page. */
export function Settings({ page: first = 'general', onClose }: { page?: SettingsPage; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const [page, setPage] = useState(first)
  const [reference, setReference] = useState('card')
  const { unit, theme, layers, screenMm } = useSettings()
  useEffect(() => ref.current!.showModal(), [])
  const [, how, w, h, r = 0] = REFERENCES[reference]
  return (
    <dialog ref={ref} className="settings" aria-label="Settings" onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <nav aria-label="Settings pages">
        <h2>Settings</h2>
        {Object.entries(PAGES).map(([p, [title, icon]]) => (
          <button key={p} type="button" aria-current={p === page ? 'page' : undefined} onClick={() => setPage(p as SettingsPage)}>
            <Icon name={icon} />
            {title}
          </button>
        ))}
      </nav>
      <div className="settings-pane">
        <header>
          <h3>{PAGES[page][0]}</h3>
          <button type="button" className="icon-button" aria-label="Close" onClick={() => ref.current!.close()}>
            <Icon name="close" />
          </button>
        </header>
        {page === 'general' && (
          <div className="settings-card">
            <Row title="Theme" note="System follows the light or dark mode of the computer">
              <Select label="Theme" value={theme} options={{ system: 'System', light: 'Light', dark: 'Dark' }} onChange={(t) => setSettings({ theme: t })} />
            </Row>
            <Row title="Unit" note="Lengths in the panels and rulers">
              <Select label="Unit" value={unit} options={UNITS} onChange={(u) => setSettings({ unit: u })} />
            </Row>
            <Row title="Layers panel" note="The layers of the current page or of its whole spread">
              <Select label="Layers panel shows" value={layers} options={{ page: 'Current page', spread: 'Current spread' }} onChange={(l) => setSettings({ layers: l })} />
            </Row>
          </div>
        )}
        {page === 'screen' && (
          <>
            <p className="settings-note">Zoom to 100% shows pages at their printed size once the screen is measured: pick something at hand and match it.</p>
            <div className="settings-card">
              <Row title="Reference" note="Something whose size you know">
                <Select label="Reference" value={reference} options={Object.fromEntries(Object.entries(REFERENCES).map(([k, [name]]) => [k, name]))} onChange={setReference} />
              </Row>
              <Row title="Screen size" note={`1 mm is ${screenMm.toFixed(2)} px`}>
                <input type="range" aria-label="Screen size" min={2} max={6} step={0.01} value={screenMm} onChange={(e) => setSettings({ screenMm: +e.currentTarget.value })} />
              </Row>
            </div>
            <p className="settings-note">{how}, then drag the slider until they match.</p>
            <div className="settings-measure">
              <div className="measure" data-bar={!h || undefined} style={{ width: w * screenMm, height: h ? h * screenMm : undefined, borderRadius: r * screenMm }} />
            </div>
          </>
        )}
        {page === 'shortcuts' && <Shortcuts />}
      </div>
    </dialog>
  )
}
