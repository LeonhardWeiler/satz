import { typeface } from './engine/engine'
import type { Editor } from './editor'
import type { Typeface } from './model'
import type { Preview, Previewed } from './exportWorker'

export type Handle = FileSystemFileHandle & { requestPermission(o: { mode: 'readwrite' }): Promise<PermissionState> }
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

/** The fonts added in this browser. */
export const storedFonts = async () => result<Uint8Array[]>((await store('fonts')).getAll())

/** Autosaves the document a second after it last changed and when the page is hidden. */
export function autosave(editor: Editor) {
  const state = () => editor.engine.version() + editor.dirty + editor.file.name
  let last = state()
  let timer = 0
  const put = async () => {
    clearTimeout(timer)
    if (state() === last) return
    last = state()
    const { name, handle } = editor.file
    ;(await store('files')).put({ bytes: editor.engine.save(), name, handle, dirty: editor.dirty } satisfies Saved, 'doc')
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

/** Sends the document to the worker of `job`, as far as it does not have it, to run `preview` or export a PDF. */
function run<T>(job: Job, editor: Editor, preview?: Preview) {
  const w = (job.worker ??= new Worker(new URL('./exportWorker.ts', import.meta.url), { type: 'module' }))
  const n = editor.snapshot.fonts.length
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
      w.terminate()
      Object.assign(job, { worker: undefined, fonts: 1, version: '' })
      fail(new Error(e.message))
    }
    w.postMessage({ doc, fonts, preview }, [...(doc ? [doc.buffer] : []), ...fonts.map((f) => f.buffer)])
  })
}

/** The document as PDF, made in a worker. */
export const pdf = (editor: Editor) => run<{ pdf: Uint8Array }>(exporter, editor).then((r) => r.pdf)

/** The inks of pages as the preflight shows them, rasterized in a worker. */
export const preview = (editor: Editor, p: Preview) => run<Previewed>(previewer, editor, p)

/** Saves into the document's file, or into one the user picks; Firefox downloads it. */
export async function save(editor: Editor, as: boolean) {
  const bytes = editor.engine.save()
  const version = editor.engine.version()
  if (!pickers.showSaveFilePicker) {
    download(bytes, editor.file.name, 'application/x-satz')
    return editor.saved(editor.file, version)
  }
  let handle = as ? null : editor.file.handle
  if (handle && (await handle.requestPermission({ mode: 'readwrite' })) !== 'granted') handle = null
  handle ??= await pickers.showSaveFilePicker({ suggestedName: editor.file.name, types: TYPES })
  const out = await handle.createWritable()
  await out.write(bytes as Uint8Array<ArrayBuffer>)
  await out.close()
  editor.saved({ name: handle.name, handle }, version)
}

export const discard = (editor: Editor) => !editor.dirty || confirm(`Discard unsaved changes to ${editor.file.name}?`)

/** Asks for a file and opens it in place of the document; `say` tells why it could not. */
export async function open(editor: Editor, say: (message: string) => void) {
  if (!discard(editor)) return
  const load = async (file: File, handle: Handle | null) => {
    try {
      editor.load(new Uint8Array(await file.arrayBuffer()), { name: file.name, handle })
    } catch (e) {
      say(`Could not open ${file.name}: ${(e as Error).message}. Choose a .satz file saved by Satz.`)
    }
  }
  if (pickers.showOpenFilePicker) {
    const [handle] = await pickers.showOpenFilePicker({ types: TYPES })
    return load(await handle.getFile(), handle)
  }
  pick('.satz', false, ([file]) => file && load(file, null))
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
  const face = editor.addFont(bytes)
  await result((await store('fonts')).put(bytes, face.hash))
}

/** Asks for TrueType and OpenType files and adds them to the fonts. */
export function addFonts(editor: Editor, say: (message: string) => void) {
  pick('.ttf,.otf', true, async (files) => {
    for (const file of files) {
      try {
        await keep(editor, new Uint8Array(await file.arrayBuffer()))
      } catch (e) {
        say(`Could not add ${file.name}: ${(e as Error).message}. Choose a .ttf or .otf file.`)
      }
    }
  })
}

/** Asks for PNG and JPEG files and places each on the current page. */
export function placeImages(editor: Editor, say: (message: string) => void) {
  pick('.png,.jpg,.jpeg', true, async (files) => {
    for (const file of files) {
      try {
        editor.placeImage(new Uint8Array(await file.arrayBuffer()), file.name)
      } catch (e) {
        say(`Could not place ${file.name}: ${(e as Error).message}. Choose a PNG or JPEG file.`)
      }
    }
  })
}

type LocalFont = { fullName: string; blob(): Promise<Blob> }
const local = window as { queryLocalFonts?: () => Promise<LocalFont[]> }
export const canFindFonts = !!local.queryLocalFonts

/** Adds the missing fonts that this computer has, by Local Font Access. */
export async function findFonts(editor: Editor, say: (message: string) => void) {
  const missing: Typeface[] = editor.snapshot.missingFonts.map((m) => m.font)
  let all: LocalFont[]
  try {
    all = await local.queryLocalFonts!()
  } catch (e) {
    return say(`Could not read the fonts on this computer: ${(e as Error).message}. Allow access to local fonts, or add them with Add font.`)
  }
  let found = 0
  for (const f of all.filter((f) => missing.some((m) => m.name === f.fullName))) {
    const bytes = new Uint8Array(await (await f.blob()).arrayBuffer())
    if (!missing.some((m) => m.hash === (typeface(bytes) as Typeface).hash)) continue
    await keep(editor, bytes)
    found++
  }
  say(`Found ${found} of ${missing.length} missing fonts on this computer.${found < missing.length ? ' Add the others with Add font.' : ''}`)
}
