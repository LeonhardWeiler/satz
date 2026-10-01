import { useSyncExternalStore } from 'react'
import { MM } from './editor'

/** Points per length unit. */
export const UNITS = { mm: MM, cm: 10 * MM, in: 72, pt: 1 } as const
export type Unit = keyof typeof UNITS

const KEY = 'satz.settings'
const DEFAULTS = { unit: 'mm' as Unit, quickEdit: true, layers: 'page' as 'page' | 'spread', keys: {} as Record<string, string> }
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

export const useSettings = () => useSyncExternalStore(subscribeSettings, () => settings)

/** `pt` in the length unit, rounded to `digits`, with the unit. */
export const length = (pt: number, digits = 1) => `${Math.round((pt / UNITS[settings.unit]) * 10 ** digits) / 10 ** digits} ${settings.unit}`
