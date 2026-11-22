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

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)

const merge = (d: unknown, v: unknown): unknown => {
  if (Array.isArray(d)) return Array.isArray(v) ? v : d
  if (!isObject(d)) return typeof v === typeof d ? v : d
  if (!isObject(v)) return d
  const keys = Object.keys(d)
  if (!keys.length) return Object.fromEntries(Object.entries(v).filter(([, x]) => typeof x === 'string'))
  return Object.fromEntries(keys.map((k) => [k, merge(d[k], v[k])]))
}

export const decode = <T>(defaults: T, json: string | null): T => {
  try {
    return merge(defaults, JSON.parse(json ?? '{}')) as T
  } catch {
    return defaults
  }
}

export const load = <T>(key: string, defaults: T): T => {
  try {
    return decode(defaults, localStorage.getItem(key))
  } catch {
    return defaults
  }
}

export const store = (key: string, value: unknown) => localStorage.setItem(key, JSON.stringify(value))

const stored = (): Settings => {
  const s = load(KEY, DEFAULTS)
  return s.unit in UNITS ? s : { ...s, unit: DEFAULTS.unit }
}

export let settings = stored()
const listeners = new Set<() => void>()
const notify = () => {
  for (const l of listeners) l()
}

export function setSettings(patch: Partial<Settings>) {
  settings = { ...settings, ...patch }
  localStorage.setItem(KEY, JSON.stringify(settings))
  notify()
}

export const subscribeSettings = (l: () => void) => {
  listeners.add(l)
  return () => void listeners.delete(l)
}

export function followBrowser() {
  addEventListener('storage', (e) => {
    if (e.key !== KEY) return
    settings = { ...DEFAULTS, ...stored() }
    notify()
  })
  const light = matchMedia('(prefers-color-scheme: light)')
  const theme = () => {
    document.documentElement.dataset.theme = settings.theme === 'system' ? (light.matches ? 'light' : 'dark') : settings.theme
    document.querySelector<HTMLMetaElement>('meta[name=theme-color]')!.content = getComputedStyle(document.documentElement).backgroundColor
  }
  theme()
  light.addEventListener('change', theme)
  listeners.add(theme)
}

export const useSettings = () => useSyncExternalStore(subscribeSettings, () => settings)

export const length = (pt: number, digits = 1) => `${Math.round((pt / UNITS[settings.unit]) * 10 ** digits) / 10 ** digits} ${settings.unit}`
