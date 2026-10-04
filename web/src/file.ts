import { useSyncExternalStore } from 'react'
import { typeface } from './engine/engine'
import type { Editor } from './editor'
import type { Typeface } from './model'
import type { Out, Preview, Previewed } from './exportWorker'

export type Handle = FileSystemFileHandle & { requestPermission(o: { mode: 'readwrite' }): Promise<PermissionState>; move?(name: string): Promise<void> }
export type Saved = { bytes: Uint8Array; name: string; handle: Handle | null; dirty: boolean }

const TYPES = [{ description: 'Satz document', accept: { 'application/x-satz': ['.satz'] } }]
const pickers = window as {
  showSaveFilePicker?: (o: { suggestedName: string; types: typeof TYPES }) => Promise<Handle>
  showOpenFilePicker?: (o: { types: typeof TYPES }) => Promise<Handle[]>
}

let db: Promise<IDBDatabase> | undefined

/** The autosaved document under 'doc', or the added fonts by hash. */
async function store(name: 'files' | 'fonts') {
  db ??= new Promise((done, fail) => {
    const r = indexedDB.open('satz', 2)
    r.onupgradeneeded = () => {
      for (const s of ['files', 'fonts']) if (!r.result.objectStoreNames.contains(s)) r.result.createObjectStore(s)
    }
    r.onsuccess = () => done(r.result)
    r.onerror = () => fail(r.error)
  })
  return (await db).transaction(name, 'readwrite').objectStore(name)
}

const result = <T,>(r: IDBRequest<T>) =>
  new Promise<T>((done, fail) => {
    r.onsuccess = () => done(r.result)
    r.onerror = () => fail(r.error)
  })

/** The document autosaved last. */
export const stored = async () => result<Saved | undefined>((await store('files')).get('doc'))

/** Keeps an autosave that could not be restored under 'doc-unreadable', which no autosave overwrites. */
export const keepUnreadable = async (saved: Saved) => result((await store('files')).put(saved, 'doc-unreadable'))

/** The fonts added in this browser. */
export const storedFonts = async () => result<Uint8Array[]>((await store('fonts')).getAll())

/** Autosaves the document a second after it last changed and when the page is hidden. */
export function autosave(editor: Editor) {
  const state = () => editor.engine.version() + editor.dirty + editor.file.name
  let last = state()
  let timer = 0
  const put = async () => {
    clearTimeout(timer)
    if (editor.broken || state() === last) return
    last = state()
    const { name, handle } = editor.file
    const files = await store('files')
    files.put({ bytes: editor.engine.save(), name, handle, dirty: editor.dirty } satisfies Saved, 'doc')
    files.transaction.commit()
  }
  const later = () => {
    clearTimeout(timer)
    timer = window.setTimeout(put, 1000)
  }
  const hidden = () => document.visibilityState === 'hidden' && put()
  const off = editor.subscribe(later)
  window.addEventListener('pagehide', put)
  document.addEventListener('visibilitychange', hidden)
  return () => {
    off()
    put()
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

/** A worker with the fonts it was sent, the bundled one from the start, and the version of the document it has. */
type Job = { worker?: Worker; fonts: number; version: string }
const exporter: Job = { fonts: 1, version: '' }
const previewer: Job = { fonts: 1, version: '' }

function reset(job: Job) {
  job.worker?.terminate()
  Object.assign(job, { worker: undefined, fonts: 1, version: '' })
}

/** Removes the added font `hash` from the document's fonts, this browser and the workers. */
export async function removeFont(editor: Editor, hash: string) {
  editor.removeFont(hash)
  reset(exporter)
  reset(previewer)
  await result((await store('fonts')).delete(hash))
}

/** Sends the document to the worker of `job`, as far as it does not have it, to run `preview` or make `out`. */
function run<T>(job: Job, editor: Editor, preview?: Preview, out?: Out) {
  const w = (job.worker ??= new Worker(new URL('./exportWorker.ts', import.meta.url), { type: 'module' }))
  const n = editor.engine.fontsAdded()
  const fonts = Array.from({ length: n - job.fonts }, (_, i) => editor.engine.font(job.fonts + i))
  const version = editor.engine.version()
  const doc = version === job.version ? null : editor.engine.save()
  return new Promise<T>((done, fail) => {
    w.onmessage = ({ data }: MessageEvent<T & { error?: string }>) => {
      if (data.error !== undefined) return fail(new Error(data.error))
      job.fonts = n
      job.version = version
      done(data)
    }
    w.onerror = (e) => {
      reset(job)
      fail(new Error(e.message))
    }
    w.postMessage({ doc, fonts, preview, out }, [...(doc ? [doc.buffer] : []), ...fonts.map((f) => f.buffer)])
  })
}

/** The files of `out`, made in a worker. */
export const exportFiles = (editor: Editor, out: Out) => run<{ files: Uint8Array[] }>(exporter, editor, undefined, out).then((r) => r.files)

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

export const discard = async (editor: Editor) => !editor.dirty || ask(`Discard unsaved changes to ${editor.file.name}?`, 'Discard')

async function load(editor: Editor, file: File, handle: Handle | null, say: (message: string) => void) {
  if (!(await discard(editor))) return
  try {
    editor.load(new Uint8Array(await file.arrayBuffer()), { name: file.name, handle })
  } catch (e) {
    say(`Could not open ${file.name}: ${(e as Error).message}. Choose a .satz file saved by Satz.`)
  }
}

/** Asks for a file and opens it in place of the document; `say` tells why it could not. */
export async function open(editor: Editor, say: (message: string) => void) {
  if (!pickers.showOpenFilePicker) return pick('.satz', false, ([file]) => file && load(editor, file, null, say))
  try {
    const [handle] = await pickers.showOpenFilePicker({ types: TYPES })
    await load(editor, await handle.getFile(), handle, say)
  } catch (e) {
    if ((e as Error).name !== 'AbortError') say(`Could not open a file: ${(e as Error).message}.`)
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
  await result((await store('fonts')).put(bytes, face.hash))
  return face
}

async function addFont(editor: Editor, file: File, say: (message: string) => void) {
  try {
    await keep(editor, new Uint8Array(await file.arrayBuffer()))
  } catch (e) {
    say(`Could not add ${file.name}: ${(e as Error).message}. Choose a .ttf, .otf, .woff or .woff2 file.`)
  }
}

async function placeImage(editor: Editor, file: File, say: (message: string) => void) {
  try {
    editor.loadImage(new Uint8Array(await file.arrayBuffer()), file.name)
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) throw e
    say(`Could not place ${file.name}: ${(e as Error).message}.`)
  }
}

/** Asks for TrueType, OpenType, WOFF and WOFF2 files and adds them to the fonts. */
export const addFonts = (editor: Editor, say: (message: string) => void) =>
  pick('.ttf,.otf,.woff,.woff2', true, async (files) => {
    for (const file of files) await addFont(editor, file, say)
  })

/** Asks for PNG and JPEG files and places each on the current page. */
export const pickProfile = (editor: Editor, say: (message: string) => void) =>
  pick('.icc,.icm', false, async ([file]) => {
    if (!file) return
    try {
      editor.setProfile(new Uint8Array(await file.arrayBuffer()))
    } catch (e) {
      say(`Could not use ${file.name}: ${(e as Error).message}.`)
    }
  })

export const placeImages = (editor: Editor, say: (message: string) => void) =>
  pick('.png,.jpg,.jpeg', true, async (files) => {
    for (const file of files) await placeImage(editor, file, say)
  })

/** Opens a dropped document, places dropped images and adds dropped fonts. */
export async function drop(editor: Editor, files: File[], say: (message: string) => void) {
  for (const file of files) {
    const ext = file.name.split('.').pop()!.toLowerCase()
    if (ext === 'satz') await load(editor, file, null, say)
    else if (['ttf', 'otf', 'woff', 'woff2'].includes(ext)) await addFont(editor, file, say)
    else if (ext === 'png' || ext === 'jpg' || ext === 'jpeg') await placeImage(editor, file, say)
    else say(`Could not use ${file.name}. Drop a .satz document, a PNG or JPEG image or a .ttf, .otf, .woff or .woff2 font.`)
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
export async function readLocalFonts(say: (message: string) => void) {
  try {
    locals = await local.queryLocalFonts!()
  } catch (e) {
    say(`Could not read the fonts on this computer: ${(e as Error).message}. Allow access to local fonts, or add them with Add font.`)
    return false
  }
  heard.forEach((f) => f())
  return true
}

if (canFindFonts) navigator.permissions.query({ name: 'local-fonts' as PermissionName }).then((p) => p.state === 'granted' && readLocalFonts(() => {}), () => {})

/** Adds the font `f` of this computer. */
export async function addLocalFont(editor: Editor, f: LocalFont) {
  try {
    return await keep(editor, new Uint8Array(await (await f.blob()).arrayBuffer()))
  } catch (e) {
    editor.say(`Could not add ${f.fullName}: ${(e as Error).message}.`)
  }
}

/** Adds the missing fonts that this computer has, by Local Font Access. */
export async function findFonts(editor: Editor, say: (message: string) => void) {
  const missing: Typeface[] = editor.snapshot.missingFonts.map((m) => m.font)
  if (!(await readLocalFonts(say))) return
  let found = 0
  for (const f of locals.filter((f) => missing.some((m) => m.name === f.fullName))) {
    const bytes = new Uint8Array(await (await f.blob()).arrayBuffer())
    if (!missing.some((m) => m.hash === (typeface(bytes) as Typeface).hash)) continue
    await keep(editor, bytes)
    found++
  }
  say(`Found ${found} of ${missing.length} missing fonts on this computer.${found < missing.length ? ' Add the others with Add font.' : ''}`)
}
