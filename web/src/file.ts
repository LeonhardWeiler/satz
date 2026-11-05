import { useSyncExternalStore } from 'react'
import { Engine, typeface } from './engine/engine'
import { UNTITLED, type Editor } from './editor'
import type { Typeface } from './model'
import type { Out, Preview, Previewed } from './exportWorker'

export type Handle = FileSystemFileHandle & { requestPermission(o: { mode: 'readwrite' }): Promise<PermissionState>; move?(name: string): Promise<void> }
export type Saved = { bytes: Uint8Array; name: string; handle: Handle | null; dirty: boolean }
export type Project = { id: string; name: string; at: number; thumb?: Uint8Array }

type Types = { description: string; accept: Record<string, string[]> }[]
const TYPES = [{ description: 'Satz document', accept: { 'application/x-satz': ['.satz'] } }]
const OPENS = [{ description: 'Satz, SVG or PDF', accept: { 'application/x-satz': ['.satz'], 'image/svg+xml': ['.svg'], 'application/pdf': ['.pdf'] } }]
const imports = (name: string) => /\.(svg|pdf)$/i.test(name)
const pickers = window as {
  showSaveFilePicker?: (o: { suggestedName: string; types: Types }) => Promise<Handle>
  showOpenFilePicker?: (o: { types: Types }) => Promise<Handle[]>
}

let db: Promise<IDBDatabase> | undefined

/** The autosaved documents by project id with their updates under [id, n], the projects by id, or the added fonts by hash. */
async function store(name: 'files' | 'fonts' | 'projects') {
  db ??= new Promise((done, fail) => {
    const r = indexedDB.open('satz', 3)
    r.onupgradeneeded = () => {
      for (const s of ['files', 'fonts', 'projects']) if (!r.result.objectStoreNames.contains(s)) r.result.createObjectStore(s)
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

/** The keys of the updates autosaved after the document of project `id`. */
const updates = (id: string) => IDBKeyRange.bound([id, 0], [id, Infinity])

/** The document of project `id` autosaved last and the updates autosaved after it. */
const stored = async (id: string) => {
  const files = await store('files')
  const [saved, after] = await Promise.all([result<Saved | undefined>(files.get(id)), result<Uint8Array[]>(files.getAll(updates(id)))])
  return saved && { ...saved, updates: after }
}

/** The projects of this browser, the last edited first; the autosave from before projects is the one under 'doc'. */
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

/** Deletes the project `id` and its autosaves. */
export async function removeProject(editor: Editor, id: string) {
  const files = await store('files')
  files.delete(id)
  files.delete(updates(id))
  await finished(files.transaction)
  await result((await store('projects')).delete(id))
  if (editor.project === id) editor.project = null
}

/** The added fonts of this browser. */
export const storedFonts = async () => result<Uint8Array[]>((await store('fonts')).getAll())

/** Writes what the autosave has not written yet. */
export let flush = () => Promise.resolve()

/** Shows the project `id`, or the projects for `null`. */
export function show(editor: Editor, id: string | null) {
  if (id) editor.project = id
  history.replaceState(null, '', id ? `#${id}` : location.pathname + location.search)
  editor.set({ home: !id })
}

/** Opens the project `id` in place of the document. */
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
  await findGrantedFonts(editor)
}

/** Opens the document `bytes` as a new project; throws when it is not one. */
export async function begin(editor: Editor, bytes: Uint8Array, file = UNTITLED) {
  await flush()
  editor.load(bytes, file)
  show(editor, crypto.randomUUID())
}

/** The first page as a PNG about 240 px on its long side. */
function thumb(editor: Editor) {
  const p = editor.snapshot.pages[0]
  return editor.engine.png(p.id, (240 * 72) / Math.max(p.width, p.height))
}

/** Autosaves the document into its project a second after it last changed and when the page is hidden:
 * the whole document when its project, file or `dirty` changed and after 100 updates, else the update since. */
export function autosave(editor: Editor) {
  const file = () => `${editor.project} ${editor.dirty} ${editor.file.name}`
  const state = () => editor.engine.version() + file()
  let last = state()
  let timer = 0
  let failed = false
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
      failed = false
    } catch (e) {
      if (!failed) editor.fail(`Autosave failed: ${(e as Error).message}. Save the file with Ctrl+S.`)
      failed = true
      timer = window.setTimeout(put, 10000)
    }
  }
  const later = () => {
    if (state() === last) return
    clearTimeout(timer)
    timer = window.setTimeout(put, 1000)
  }
  const hidden = () => document.visibilityState === 'hidden' && put()
  const off = editor.subscribe(later)
  window.addEventListener('pagehide', put)
  document.addEventListener('visibilitychange', hidden)
  flush = put
  return () => {
    off()
    put()
    flush = () => Promise.resolve()
    window.removeEventListener('pagehide', put)
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

/** A worker with the fonts it was sent, the bundled one from the start, the version and version vector of the document it has and its last job, which the next waits for. */
type Job = { worker?: Worker; fonts: number; version: string; seen: Uint8Array; queue: Promise<unknown> }
const exporter: Job = { fonts: 1, version: '', seen: new Uint8Array(), queue: Promise.resolve() }
const previewer: Job = { fonts: 1, version: '', seen: new Uint8Array(), queue: Promise.resolve() }

function reset(job: Job) {
  job.worker?.terminate()
  Object.assign(job, { worker: undefined, fonts: 1, version: '', seen: new Uint8Array(), queue: Promise.resolve() })
}

/** Removes the added font `hash` from the document's fonts, this browser and the workers. */
export async function removeFont(editor: Editor, hash: string) {
  editor.removeFont(hash)
  reset(exporter)
  reset(previewer)
  await result((await store('fonts')).delete(hash))
}

/** Sends the document to the worker of `job`, as far as it does not have it, to run `preview` or make `out` once its last job is done. */
function run<T>(job: Job, editor: Editor, preview?: Preview, out?: Out) {
  const next = job.queue.then(() => {
    const w = (job.worker ??= new Worker(new URL('./exportWorker.ts', import.meta.url), { type: 'module' }))
    const n = editor.engine.fontsAdded()
    const fonts = Array.from({ length: n - job.fonts }, (_, i) => editor.engine.font(job.fonts + i))
    const version = editor.engine.version()
    const seen = editor.engine.versionVector()
    const doc = version === job.version ? null : editor.engine.updates(job.seen)
    return new Promise<T>((done, fail) => {
      w.onmessage = ({ data }: MessageEvent<T & { error?: string }>) => {
        if (data.error !== undefined) {
          reset(job)
          return fail(new Error(data.error))
        }
        job.fonts = n
        job.version = version
        job.seen = seen
        done(data)
      }
      w.onerror = (e) => {
        reset(job)
        fail(new Error(e.message))
      }
      w.postMessage({ doc, fonts, preview, out }, [...(doc ? [doc.buffer] : []), ...fonts.map((f) => f.buffer)])
    })
  })
  job.queue = next.catch(() => {})
  return next
}

/** The files of `out`, made in a worker. */
export const exportFile = (editor: Editor, out: Out) => run<{ file: Uint8Array }>(exporter, editor, undefined, out).then((r) => r.file)

/** The inks of pages as the preflight shows them, rasterized in a worker. */
export const preview = (editor: Editor, p: Preview) => run<Previewed>(previewer, editor, p)

/** Saves into the document's file, renamed to the document's name, or into one the user picks; Firefox downloads it. */
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

/** Asks `question` in a modal dialog; resolves true when `yes` is chosen. */
export function ask(question: string, yes: string): Promise<boolean> {
  const d = document.createElement('dialog')
  d.className = 'ask'
  d.innerHTML = '<form method="dialog"><p></p><div><button value="" autofocus>Cancel</button><button value="yes" class="primary"></button></div></form>'
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

/** Whether the document may go: it is autosaved into its project or saved, or the user says so. */
export const discard = async (editor: Editor) =>
  !editor.dirty || (editor.autosaves && !!editor.project) || ask(`Discard unsaved changes to ${editor.file.name}?`, 'Discard')

async function load(editor: Editor, file: File, handle: Handle | null) {
  if (!(await discard(editor))) return
  try {
    const bytes = new Uint8Array(await file.arrayBuffer())
    if (imports(file.name)) await begin(editor, Engine.importFile(bytes, file.name), { name: file.name.replace(/\.\w+$/, '.satz'), handle: null })
    else await begin(editor, bytes, { name: file.name, handle })
  } catch (e) {
    return editor.fail(`Could not open ${file.name}: ${(e as Error).message}. Choose a .satz file saved by Satz, an SVG or a PDF.`)
  }
  await findGrantedFonts(editor)
}

/** Asks for a document, SVG or PDF and opens it as a new project. */
export async function open(editor: Editor) {
  if (!pickers.showOpenFilePicker) return pick('.satz,.svg,.pdf', false, ([file]) => file && load(editor, file, null))
  try {
    const [handle] = await pickers.showOpenFilePicker({ types: OPENS })
    await load(editor, await handle.getFile(), handle)
  } catch (e) {
    if ((e as Error).name !== 'AbortError') editor.fail(`Could not open a file: ${(e as Error).message}.`)
  }
}

/** One file input for every picker, held so that it is not collected before it fires `change`. */
const input = document.createElement('input')
input.type = 'file'

function pick(accept: string, multiple: boolean, picked: (files: File[]) => void) {
  input.accept = accept
  input.multiple = multiple
  input.value = ''
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

async function placeImage(editor: Editor, file: File) {
  try {
    const bytes = new Uint8Array(await file.arrayBuffer())
    if (!imports(file.name)) return editor.loadImage(bytes, file.name)
    editor.engine.takeClip(Engine.importFile(bytes, file.name))
    editor.clip = null
    editor.paste()
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    editor.fail(`Could not place ${file.name}: ${(e as Error).message}.`)
  }
}

/** Asks for TrueType, OpenType, WOFF and WOFF2 files and adds them to the fonts. */
export const addFonts = (editor: Editor) =>
  pick('.ttf,.otf,.woff,.woff2', true, async (files) => {
    for (const file of files) await addFont(editor, file)
  })

/** Asks for an ICC profile and makes it the document's CMYK profile. */
export const pickProfile = (editor: Editor) =>
  pick('.icc,.icm', false, async ([file]) => {
    if (!file) return
    try {
      editor.setProfile(new Uint8Array(await file.arrayBuffer()))
    } catch (e) {
      editor.fail(`Could not use ${file.name}: ${(e as Error).message}.`)
    }
  })

/** Asks for PNG, JPEG, SVG and PDF files and places each on the current page, an SVG or PDF with its first page's layers. */
export const placeImages = (editor: Editor) =>
  pick('.png,.jpg,.jpeg,.svg,.pdf', true, async (files) => {
    for (const file of files) await placeImage(editor, file)
  })

/** Opens a dropped document, places dropped images, SVGs and PDFs (opens them on the projects) and adds dropped fonts. */
export async function drop(editor: Editor, files: File[]) {
  for (const file of files) {
    const ext = file.name.split('.').pop()!.toLowerCase()
    if (ext === 'satz' || (editor.home && imports(file.name))) await load(editor, file, null)
    else if (['ttf', 'otf', 'woff', 'woff2'].includes(ext)) await addFont(editor, file)
    else if (!editor.home && ['png', 'jpg', 'jpeg', 'svg', 'pdf'].includes(ext)) await placeImage(editor, file)
    else editor.fail(`Could not use ${file.name}. Drop a .satz document, a PNG, JPEG, SVG or PDF or a .ttf, .otf, .woff or .woff2 font.`)
  }
}

type LocalFont = { fullName: string; family: string; style: string; postscriptName: string; blob(): Promise<Blob> }
const local = window as { queryLocalFonts?: () => Promise<LocalFont[]> }
export const canFindFonts = !!local.queryLocalFonts
let locals: LocalFont[] = []
const heard = new Set<() => void>()

/** The fonts of this computer, once read by `readLocalFonts`. */
export const useLocalFonts = () =>
  useSyncExternalStore(
    (f) => {
      heard.add(f)
      return () => heard.delete(f)
    },
    () => locals,
  )

/** Reads the fonts of this computer by Local Font Access; says why it could not. */
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

/** Adds the font `f` of this computer. */
export async function addLocalFont(editor: Editor, f: LocalFont) {
  try {
    return await keep(editor, new Uint8Array(await (await f.blob()).arrayBuffer()))
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    editor.fail(`Could not add ${f.fullName}: ${(e as Error).message}.`)
  }
}

/** Adds the missing fonts that this computer has, by Local Font Access. */
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

/** Adds the missing fonts that this computer has, if Satz may already read them. */
export async function findGrantedFonts(editor: Editor) {
  if (!canFindFonts || !editor.snapshot.missingFonts.length) return
  const p = await navigator.permissions.query({ name: 'local-fonts' as PermissionName }).catch(() => null)
  if (p?.state === 'granted') await findFonts(editor)
}
