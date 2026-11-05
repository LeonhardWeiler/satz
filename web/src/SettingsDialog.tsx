import { useEffect, useRef } from 'react'
import { Select } from './controls'
import { Icon } from './icons'
import { setSettings, useSettings, type Unit } from './settings'

const NAMES: Record<Unit, string> = { mm: 'Millimeters', cm: 'Centimeters', in: 'Inches', pt: 'Points' }

/** Ctrl+,: the settings of this browser. */
export function Settings({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const { unit, theme, quickEdit, layers, screenMm } = useSettings()
  useEffect(() => {
    ref.current!.showModal()
    ref.current!.focus()
  }, [])
  return (
    <dialog ref={ref} className="help settings" aria-label="Settings" tabIndex={-1} onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <header className="help-head">
        <h2>Settings</h2>
        <span className="grow" />
        <button type="button" className="icon-button" aria-label="Close" onClick={() => ref.current!.close()}>
          <Icon name="close" />
        </button>
      </header>
      <div className="help-body settings-body">
        <div className="settings-row">
          Theme
          <Select label="Theme" value={theme} options={{ system: 'System', light: 'Light', dark: 'Dark' }} onChange={(t) => setSettings({ theme: t })} />
        </div>
        <div className="settings-row">
          Unit
          <Select label="Unit" value={unit} options={NAMES} onChange={(u) => setSettings({ unit: u })} />
        </div>
        <div className="settings-row">
          Layers panel shows
          <Select label="Layers panel shows" value={layers} options={{ page: 'Current page', spread: 'Current spread' }} onChange={(l) => setSettings({ layers: l })} />
        </div>
        <label className="settings-row">
          Screen size
          <input type="range" aria-label="Screen size" min={2} max={6} step={0.01} value={screenMm} onChange={(e) => setSettings({ screenMm: +e.currentTarget.value })} />
        </label>
        <div className="card-bar" style={{ width: 85.6 * screenMm, height: 53.98 * screenMm, borderRadius: 3.18 * screenMm }}>
          Hold a bank card here and match its size for Zoom to 100%
        </div>
        <label className="settings-row">
          Quick edit bar above the selection
          <input type="checkbox" checked={quickEdit} onChange={(e) => setSettings({ quickEdit: e.currentTarget.checked })} />
        </label>
      </div>
    </dialog>
  )
}
