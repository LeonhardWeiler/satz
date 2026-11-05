import { useSyncExternalStore } from 'react'
import { MM } from './editor'

/** Points per length unit. */
export const UNITS = { mm: MM, cm: 10 * MM, in: 72, pt: 1 } as const
export type Unit = keyof typeof UNITS
/** The largest length in pt, as in the engine. */
export const MAX_LENGTH = 20000 * MM

const KEY = 'satz.settings'
const DEFAULTS = {
  unit: 'mm' as Unit,
  theme: 'system' as 'system' | 'light' | 'dark',
  quickEdit: true,
  /** CSS px in 1 mm of this screen, for Zoom to 100%. */
  screenMm: 96 / 25.4,
  layers: 'page' as 'page' | 'spread',
  keys: {} as Record<string, string>,
  /** The width of the masters in the page overview, its page height and columns of spreads, 0 to fill the width, all in px. */
  overview: { masters: 170, pages: 200, columns: 0 },
}
export type Settings = typeof DEFAULTS

const stored = (): Partial<Settings> => {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}')
  } catch {
    return {}
  }
}

/** The settings of this browser, kept in local storage. */
export let settings: Settings = { ...DEFAULTS, ...stored() }
const listeners = new Set<() => void>()

export function setSettings(patch: Partial<Settings>) {
  settings = { ...settings, ...patch }
  localStorage.setItem(KEY, JSON.stringify(settings))
  for (const l of listeners) l()
}

export const subscribeSettings = (l: () => void) => {
  listeners.add(l)
  return () => void listeners.delete(l)
}

const light = matchMedia('(prefers-color-scheme: light)')
const theme = () => (document.documentElement.dataset.theme = settings.theme === 'system' ? (light.matches ? 'light' : 'dark') : settings.theme)
theme()
light.addEventListener('change', theme)
listeners.add(theme)

export const useSettings = () => useSyncExternalStore(subscribeSettings, () => settings)

/** `pt` in the length unit, rounded to `digits`, with the unit. */
export const length = (pt: number, digits = 1) => `${Math.round((pt / UNITS[settings.unit]) * 10 ** digits) / 10 ** digits} ${settings.unit}`
