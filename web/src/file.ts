import { useSyncExternalStore } from 'react'
import { typeface } from './engine/engine'
import { UNTITLED, type Editor } from './editor'
import type { Typeface } from './model'
import type { Out, Preview, Previewed, Source } from './exportWorker'

export type Handle = FileSystemFileHandle & {
  queryPermission(o: { mode: 'read' }): Promise<PermissionState>
  requestPermission(o: { mode: 'read' | 'readwrite' }): Promise<PermissionState>
  move?(name: string): Promise<void>
}
export type Saved = { bytes: Uint8Array; name: string; handle: Handle | null; dirty: boolean }
export type Project = { id: string; name: string; at: number; thumb?: Uint8Array }

type Types = { description: string; accept: Record<string, string[]> }[]
const TYPES = [{ description: 'Satz document', accept: { 'application/x-satz': ['.satz'] } }]
const OPENS = [{ description: 'Satz, SVG, PDF or Figma', accept: { 'application/x-satz': ['.satz'], 'image/svg+xml': ['.svg'], 'application/pdf': ['.pdf'], 'application/x-figma': ['.fig'] } }]
const IMAGES = [{ description: 'PNG or JPEG image', accept: { 'image/png': ['.png'], 'image/jpeg': ['.jpg', '.jpeg'] } }]
const PLACES = [{ description: 'Image, SVG, PDF or Figma', accept: { ...IMAGES[0].accept, 'image/svg+xml': ['.svg'], 'application/pdf': ['.pdf'], 'application/x-figma': ['.fig'] } }]
const imports = (name: string) => /\.(svg|pdf|fig)$/i.test(name)
const pickers = globalThis as {
  showSaveFilePicker?: (o: { suggestedName: string; types: Types }) => Promise<Handle>
  showOpenFilePicker?: (o: { types: Types; multiple?: boolean }) => Promise<Handle[]>
}

let db: Promise<IDBDatabase> | undefined

async function store(name: 'files' | 'fonts' | 'projects' | 'links') {
  db ??= new Promise((done, fail) => {
    const r = indexedDB.open('satz', 4)
    r.onupgradeneeded = () => {
      for (const s of ['files', 'fonts', 'projects', 'links']) if (!r.result.objectStoreNames.contains(s)) r.result.createObjectStore(s)
    }
    r.onsuccess = () => done(r.result)
    r.onerror = () => fail(r.error)
  })
  return (await db).transaction(name, 'readwrite').objectStore(name)
}

const finished = (t: IDBTransaction) =>
  new Promise<void>((done, fail) => {
    t.oncomplete = () => done()
    t.onerror = t.onabort = () => fail(t.error ?? new Error('the write was aborted'))
  })

const result = <T,>(r: IDBRequest<T>) =>
  new Promise<T>((done, fail) => {
    r.onsuccess = () => done(r.result)
    r.onerror = () => fail(r.error)
  })

const updates = (id: string) => IDBKeyRange.bound([id, 0], [id, Infinity])

const stored = async (id: string) => {
  const files = await store('files')
  const [saved, after] = await Promise.all([result<Saved | undefined>(files.get(id)), result<Uint8Array[]>(files.getAll(updates(id)))])
  return saved && { ...saved, updates: after }
}

export async function projects() {
  const all = await result<Project[]>((await store('projects')).getAll())
  const old = !all.some((p) => p.id === 'doc') && (await stored('doc'))
  if (old) {
    const p = { id: 'doc', name: old.name, at: Date.now() }
    await result((await store('projects')).put(p, p.id))
    all.push(p)
  }
  return all.sort((a, b) => b.at - a.at)
}

export async function removeProject(editor: Editor, id: string) {
  const files = await store('files')
  files.delete(id)
  files.delete(updates(id))
  await finished(files.transaction)
  await result((await store('projects')).delete(id))
  if (editor.project === id) editor.set({ project: null })
}

export const storedFonts = async () => result<Uint8Array[]>((await store('fonts')).getAll())

export let flush = () => Promise.resolve()

export function show(editor: Editor, id: string | null) {
  history.replaceState(null, '', id ? `#${id}` : location.pathname + location.search)
  editor.set(id ? { project: id, home: false } : { home: true })
}

export async function openProject(editor: Editor, id: string) {
  await flush()
  const saved = await stored(id).catch(() => undefined)
  if (!saved) return editor.fail('Could not open the project: it is not in this browser.')
  try {
    editor.load(saved.bytes, { name: saved.name, handle: saved.handle }, saved.dirty, saved.updates)
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    return editor.fail(`Could not open ${saved.name}: ${(e as Error).message}.`)
  }
  show(editor, id)
  await findGranted(editor)
}

export async function begin(editor: Editor, bytes: Uint8Array, file = UNTITLED) {
  await flush()
  editor.load(bytes, file)
  show(editor, crypto.randomUUID())
}

function thumb(editor: Editor) {
  const p = editor.snapshot.pages[0]
  return editor.engine.png(p.id, (240 * 72) / Math.max(p.width, p.height))
}

export function autosave(editor: Editor) {
  const file = () => `${editor.project} ${editor.dirty} ${editor.file.name}`
  const state = () => editor.engine.version() + file()
  let last = state()
  let timer = 0
  let seen: Uint8Array | undefined
  let base = ''
  let appended = 0
  const put = async () => {
    clearTimeout(timer)
    const now = state()
    const id = editor.project
    if (!editor.autosaves || !id || now === last) return
    const { name, handle } = editor.file
    try {
      const files = await store('files')
      const vv = editor.engine.versionVector()
      const whole = !seen || base !== file() || appended >= 100
      if (whole) {
        files.put({ bytes: editor.engine.updates(new Uint8Array()), name, handle, dirty: editor.dirty } satisfies Saved, id)
        files.delete(updates(id))
      } else files.put(editor.engine.updates(seen!), [id, appended + 1])
      files.transaction.commit()
      await finished(files.transaction)
      await result((await store('projects')).put({ id, name, at: Date.now(), thumb: thumb(editor) } satisfies Project, id))
      appended = whole ? 0 : appended + 1
      base = file()
      seen = vv
      last = now
      if (editor.autosaveFailed) editor.set({ autosaveFailed: false })
    } catch (e) {
      if (!editor.autosaveFailed) {
        editor.fail(`Autosave failed: ${(e as Error).message}. Save the file with Ctrl+S.`)
        editor.set({ autosaveFailed: true })
      }
      clearTimeout(timer)
      timer = window.setTimeout(put, 10000)
    }
  }
  const later = () => {
    if (state() === last) return
    clearTimeout(timer)
    timer = window.setTimeout(put, 1000)
  }
  const hidden = () => document.visibilityState === 'hidden' && put()
  const leave = (e: BeforeUnloadEvent) => kept(editor) || e.preventDefault()
  const off = editor.subscribe(later)
  window.addEventListener('pagehide', put)
  window.addEventListener('beforeunload', leave)
  document.addEventListener('visibilitychange', hidden)
  flush = put
  return () => {
    off()
    put()
    flush = () => Promise.resolve()
    window.removeEventListener('pagehide', put)
    window.removeEventListener('beforeunload', leave)
    document.removeEventListener('visibilitychange', hidden)
  }
}

export function download(bytes: Uint8Array, name: string, type: string) {
  const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type }))
  const a = document.createElement('a')
  a.href = url
  a.download = name
  a.click()
  URL.revokeObjectURL(url)
}

type Job = { worker?: Worker; fonts: number; images: Set<string>; version: string; seen: Uint8Array; queue: Promise<unknown> }
const fresh = () => ({ fonts: 0, images: new Set<string>(), version: '', seen: new Uint8Array(), queue: Promise.resolve() })
const exporter: Job = fresh()
const previewer: Job = fresh()

function reset(job: Job) {
  job.worker?.terminate()
  Object.assign(job, { worker: undefined, ...fresh() })
}

export async function removeFont(editor: Editor, hash: string) {
  editor.removeFont(hash)
  reset(exporter)
  reset(previewer)
  await result((await store('fonts')).delete(hash))
}

function spawn(job: Job) {
  const w = new Worker(new URL('./exportWorker.ts', import.meta.url), { type: 'module' })
  w.onerror = () => reset(job)
  return w
}

export const warm = () => void (exporter.worker ??= spawn(exporter))

function run<T>(job: Job, editor: Editor, task: { preview?: Preview; out?: Out; source?: Source }) {
  const next = job.queue.then(() => {
    const w = (job.worker ??= spawn(job))
    const n = editor.engine.fontsAdded()
    const fonts = Array.from({ length: n - job.fonts }, (_, i) => editor.engine.font(job.fonts + i))
    const version = editor.engine.version()
    const seen = editor.engine.versionVector()
    const doc = version === job.version ? null : editor.engine.updates(job.seen)
    const { links, images: read } = editor.snapshot
    const hashes = Object.keys(links).filter((h) => h in read && !job.images.has(h))
    const images = hashes.map((h) => editor.engine.imageFile(h)!)
    return new Promise<T>((done, fail) => {
      w.onmessage = ({ data }: MessageEvent<T & { error?: string }>) => {
        if (data.error !== undefined) {
          reset(job)
          return fail(new Error(data.error))
        }
        job.fonts = n
        hashes.forEach((h) => job.images.add(h))
        job.version = version
        job.seen = seen
        done(data)
      }
      w.onerror = (e) => {
        reset(job)
        fail(new Error(e.message))
      }
      w.postMessage({ doc, fonts, images, ...task }, [...(doc ? [doc.buffer] : []), ...fonts.map((f) => f.buffer), ...images.map((i) => i.buffer)])
    })
  })
  job.queue = next.catch(() => {})
  return next
}

export const exportFile = (editor: Editor, out: Out) => run<{ file: Uint8Array }>(exporter, editor, { out }).then((r) => r.file)

const importFile = (editor: Editor, bytes: Uint8Array, name: string) => run<{ file: Uint8Array }>(exporter, editor, { source: { bytes, name } }).then((r) => r.file)

export const preview = (editor: Editor, p: Preview) => run<Previewed>(previewer, editor, { preview: p })

export async function save(editor: Editor, as: boolean) {
  const bytes = editor.engine.save()
  const version = editor.engine.version()
  const { name } = editor.file
  if (!pickers.showSaveFilePicker) {
    download(bytes, name, 'application/x-satz')
    return editor.saved(editor.file, version)
  }
  let handle = as ? null : editor.file.handle
  if (handle && (await handle.requestPermission({ mode: 'readwrite' })) !== 'granted') handle = null
  if (handle && handle.name !== name) await handle.move?.(name).catch(() => {})
  if (handle?.name !== name) handle = null
  handle ??= await pickers.showSaveFilePicker({ suggestedName: name, types: TYPES })
  const out = await handle.createWritable()
  await out.write(bytes as Uint8Array<ArrayBuffer>)
  await out.close()
  editor.saved({ name: handle.name, handle }, version)
}

export function ask(question: string, yes: string): Promise<boolean> {
  const d = document.createElement('dialog')
  d.className = 'ask'
  d.innerHTML = '<form method="dialog"><p></p><div><button value="" class="button" autofocus>Cancel</button><button value="yes" class="primary"></button></div></form>'
  d.querySelector('p')!.textContent = question
  d.querySelector('.primary')!.textContent = yes
  document.body.append(d)
  d.showModal()
  return new Promise((done) =>
    d.addEventListener('close', () => {
      d.remove()
      done(d.returnValue === 'yes')
    }),
  )
}

export const autosaving = (editor: Editor) => editor.autosaves && !editor.autosaveFailed

const kept = (editor: Editor) => !editor.dirty || (autosaving(editor) && !!editor.project)

export const discard = async (editor: Editor) =>
  kept(editor) || ask(`Discard unsaved changes to ${editor.file.name}?`, 'Discard')

async function load(editor: Editor, file: File, handle: Handle | null) {
  if (!(await discard(editor))) return
  try {
    const bytes = new Uint8Array(await file.arrayBuffer())
    if (imports(file.name)) await begin(editor, await importFile(editor, bytes, file.name), { name: file.name.replace(/\.\w+$/, '.satz'), handle: null })
    else await begin(editor, bytes, { name: file.name, handle })
  } catch (e) {
    return editor.fail(`Could not open ${file.name}: ${(e as Error).message}. Choose a .satz file saved by Satz, an SVG, a PDF or a Figma file.`)
  }
  await findGranted(editor)
}

export async function open(editor: Editor) {
  if (!pickers.showOpenFilePicker) return pick('.satz,.svg,.pdf,.fig', false, ([file]) => file && load(editor, file, null))
  try {
    const [handle] = await pickers.showOpenFilePicker({ types: OPENS })
    await load(editor, await handle.getFile(), handle)
  } catch (e) {
    if ((e as Error).name !== 'AbortError') editor.fail(`Could not open a file: ${(e as Error).message}.`)
  }
}

function pick(accept: string, multiple: boolean, picked: (files: File[]) => void) {
  const input = Object.assign(document.createElement('input'), { type: 'file', accept, multiple })
  input.onchange = () => picked([...(input.files ?? [])])
  input.click()
}

async function keep(editor: Editor, bytes: Uint8Array) {
  const face: Typeface = editor.addFont(bytes)
  await store('fonts').then((s) => result(s.put(bytes, face.hash))).catch((e: Error) => editor.fail(`${face.name} is used now but not kept for the next visit: ${e.message}.`))
  return face
}

async function addFont(editor: Editor, file: File) {
  try {
    await keep(editor, new Uint8Array(await file.arrayBuffer()))
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    editor.fail(`Could not add ${file.name}: ${(e as Error).message}. Choose a .ttf, .otf, .woff or .woff2 file.`)
  }
}

async function placeImage(editor: Editor, file: File, handle: Handle | null = null) {
  try {
    const bytes = new Uint8Array(await file.arrayBuffer())
    if (!imports(file.name) && handle && editor.snapshot.linkImages) {
      const hash = editor.loadImage(bytes, file.name, true)
      return await result((await store('links')).put(handle, hash))
    }
    if (!imports(file.name)) return void editor.loadImage(bytes, file.name)
    editor.engine.takeClip(await importFile(editor, bytes, file.name))
    editor.set({ clip: null })
    editor.paste()
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    editor.fail(`Could not place ${file.name}: ${(e as Error).message}.`)
  }
}

export const addFonts = (editor: Editor) =>
  pick('.ttf,.otf,.woff,.woff2', true, async (files) => {
    for (const file of files) await addFont(editor, file)
  })

export const pickProfile = (editor: Editor) =>
  pick('.icc,.icm', false, async ([file]) => {
    if (!file) return
    try {
      editor.setProfile(new Uint8Array(await file.arrayBuffer()))
    } catch (e) {
      editor.fail(`Could not use ${file.name}: ${(e as Error).message}.`)
    }
  })

export async function placeImages(editor: Editor) {
  if (!editor.snapshot.linkImages || !pickers.showOpenFilePicker)
    return pick('.png,.jpg,.jpeg,.svg,.pdf,.fig', true, async (files) => {
      for (const file of files) await placeImage(editor, file)
    })
  try {
    for (const handle of await pickers.showOpenFilePicker({ types: PLACES, multiple: true })) await placeImage(editor, await handle.getFile(), handle)
  } catch (e) {
    if ((e as Error).name !== 'AbortError') editor.fail(`Could not place a file: ${(e as Error).message}.`)
  }
}

export async function drop(editor: Editor, files: File[], handles: Promise<Handle | null>[] = []) {
  for (const [i, file] of files.entries()) {
    const ext = file.name.split('.').pop()!.toLowerCase()
    if (ext === 'satz' || (editor.home && imports(file.name))) await load(editor, file, null)
    else if (['ttf', 'otf', 'woff', 'woff2'].includes(ext)) await addFont(editor, file)
    else if (!editor.home && ['png', 'jpg', 'jpeg', 'svg', 'pdf', 'fig'].includes(ext)) await placeImage(editor, file, await handles[i]?.catch(() => null))
    else editor.fail(`Could not use ${file.name}. Drop a .satz document, a PNG, JPEG, SVG, PDF or Figma file or a .ttf, .otf, .woff or .woff2 font.`)
  }
}

type LocalFont = { fullName: string; family: string; style: string; postscriptName: string; blob(): Promise<Blob> }
const local = globalThis as { queryLocalFonts?: () => Promise<LocalFont[]> }
export const canFindFonts = !!local.queryLocalFonts
let locals: LocalFont[] = []
const heard = new Set<() => void>()

export const useLocalFonts = () =>
  useSyncExternalStore(
    (f) => {
      heard.add(f)
      return () => heard.delete(f)
    },
    () => locals,
  )

export async function readLocalFonts(editor?: Editor) {
  try {
    locals = await local.queryLocalFonts!()
  } catch (e) {
    editor?.fail(`Could not read the fonts on this computer: ${(e as Error).message}. Allow access to local fonts, or add them with Add font.`)
    return false
  }
  heard.forEach((f) => f())
  return true
}

if (canFindFonts) navigator.permissions.query({ name: 'local-fonts' as PermissionName }).then((p) => p.state === 'granted' && readLocalFonts(), () => {})

export async function addLocalFont(editor: Editor, f: LocalFont) {
  try {
    return await keep(editor, new Uint8Array(await (await f.blob()).arrayBuffer()))
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    editor.fail(`Could not add ${f.fullName}: ${(e as Error).message}.`)
  }
}

export async function findFonts(editor: Editor) {
  const missing: Typeface[] = editor.snapshot.missingFonts.map((m) => m.font)
  if (!(await readLocalFonts(editor))) return
  let found = 0
  for (const f of locals.filter((f) => missing.some((m) => m.name === f.fullName))) {
    const bytes = new Uint8Array(await (await f.blob()).arrayBuffer())
    if (!missing.some((m) => m.hash === (typeface(bytes) as Typeface).hash)) continue
    await keep(editor, bytes)
    found++
  }
  editor.say(`Found ${found} of ${missing.length} missing fonts on this computer.${found < missing.length ? ' Add the others with Add font.' : ''}`)
}

export async function findGranted(editor: Editor) {
  const { links, images } = editor.snapshot
  for (const hash of Object.keys(links).filter((h) => !(h in images))) {
    const handle = await link(hash)
    const file = handle && (await handle.queryPermission({ mode: 'read' })) === 'granted' && (await handle.getFile().catch(() => null))
    if (file) await readLink(editor, hash, file, handle).catch((e: Error) => editor.fail(`Could not read ${file.name}: ${e.message}.`))
  }
  if (!canFindFonts || !editor.snapshot.missingFonts.length) return
  const p = await navigator.permissions.query({ name: 'local-fonts' as PermissionName }).catch(() => null)
  if (p?.state === 'granted') await findFonts(editor)
}

const link = (hash: string) => store('links').then((s) => result<Handle | undefined>(s.get(hash)), () => undefined)

async function readLink(editor: Editor, hash: string, file: File, handle: Handle | null) {
  const bytes = new Uint8Array(await file.arrayBuffer())
  const to = editor.registerImage(bytes)
  if (to !== hash) editor.apply({ type: 'relink', image: hash, to, name: file.name })
  if (handle) await result((await store('links')).put(handle, to))
}

export async function relink(editor: Editor, hash: string) {
  const failed = (e: Error) => e.name !== 'AbortError' && editor.fail(`Could not read the image: ${e.message}.`)
  if (!pickers.showOpenFilePicker) return pick('.png,.jpg,.jpeg', false, ([file]) => file && readLink(editor, hash, file, null).catch(failed))
  try {
    const kept = await link(hash)
    const file = kept && (await kept.requestPermission({ mode: 'read' })) === 'granted' && (await kept.getFile().catch(() => null))
    if (file) return await readLink(editor, hash, file, kept)
    const [handle] = await pickers.showOpenFilePicker({ types: IMAGES })
    await readLink(editor, hash, await handle.getFile(), handle)
  } catch (e) {
    failed(e as Error)
  }
}

export const canLink = !!pickers.showOpenFilePicker
