import { useSyncExternalStore } from 'react'
import { UNITS } from './field'
import { MM, type ColorMode, type Grid } from './types'

export { UNITS }
export type Unit = keyof typeof UNITS
export const MAX_LENGTH = 20000 * MM

const KEY = 'satz.settings'
const DEFAULTS = {
  unit: 'mm' as Unit,
  theme: 'system' as 'system' | 'light' | 'dark',
  screenMm: 96 / 25.4,
  layers: 'page' as 'page' | 'spread',
  keys: {} as Record<string, string>,
  overview: { masters: 170, pages: 200, columns: 0 },
  docPresets: [] as DocPreset[],
  gridPresets: [] as { name: string; grids: Grid[] }[],
}
export type DocPreset = { name: string; w: number; h: number; pages: number; facing: boolean; mode: ColorMode; margin: number; columns: number; gutter: number; bleed: number }
export type Settings = typeof DEFAULTS

const stored = (): Partial<Settings> => {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}')
  } catch {
    return {}
  }
}

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

export const length = (pt: number, digits = 1) => `${Math.round((pt / UNITS[settings.unit]) * 10 ** digits) / 10 ** digits} ${settings.unit}`
