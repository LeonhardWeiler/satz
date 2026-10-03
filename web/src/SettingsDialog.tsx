import { useEffect, useRef } from 'react'
import { Check, Select } from './controls'
import { Icon } from './icons'
import { setSettings, useSettings, type Unit } from './settings'

const NAMES: Record<Unit, string> = { mm: 'Millimetres', cm: 'Centimetres', in: 'Inches', pt: 'Points' }

/** Ctrl+,: the settings of this browser. */
export function Settings({ onClose }: { onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null)
  const { unit, quickEdit, layers } = useSettings()
  useEffect(() => {
    ref.current!.showModal()
  }, [])
  return (
    <dialog ref={ref} className="help settings" aria-label="Settings" onClose={onClose} onClick={(e) => e.target === e.currentTarget && ref.current!.close()}>
      <header className="help-head">
        <h2>Settings</h2>
        <span className="grow" />
        <button type="button" className="icon-button" aria-label="Close" onClick={() => ref.current!.close()}>
          <Icon name="close" />
        </button>
      </header>
      <div className="help-body settings-body">
        <div className="settings-row">
          Unit
          <Select label="Unit" value={unit} options={NAMES} onChange={(u) => setSettings({ unit: u })} />
        </div>
        <div className="settings-row">
          Layers
          <Select label="Layers" value={layers} options={{ page: 'Of the page', spread: 'Of the spread' }} onChange={(l) => setSettings({ layers: l })} />
        </div>
        <Check label="Quick edit bar above the selection" value={quickEdit} set={(quickEdit) => setSettings({ quickEdit })} />
      </div>
    </dialog>
  )
}
