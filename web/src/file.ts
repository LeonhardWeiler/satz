import { Engine } from './engine/engine'
import type { Editor } from './editor'

export type Handle = FileSystemFileHandle & { requestPermission(o: { mode: 'readwrite' }): Promise<PermissionState> }
export type Saved = { bytes: Uint8Array; name: string; handle: Handle | null; dirty: boolean }

const TYPES = [{ description: 'Satz document', accept: { 'application/x-satz': ['.satz'] } }]
const pickers = window as {
  showSaveFilePicker?: (o: { suggestedName: string; types: typeof TYPES }) => Promise<Handle>
  showOpenFilePicker?: (o: { types: typeof TYPES }) => Promise<Handle[]>
}

let db: Promise<IDBDatabase> | undefined

async function files() {
  db ??= new Promise((done, fail) => {
    const r = indexedDB.open('satz', 1)
    r.onupgradeneeded = () => r.result.createObjectStore('files')
    r.onsuccess = () => done(r.result)
    r.onerror = () => fail(r.error)
  })
  return (await db).transaction('files', 'readwrite').objectStore('files')
}

/** The document autosaved last. */
export async function stored() {
  const r = (await files()).get('doc')
  return new Promise<Saved | undefined>((done, fail) => {
    r.onsuccess = () => done(r.result)
    r.onerror = () => fail(r.error)
  })
}

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
    ;(await files()).put({ bytes: editor.engine.save(), name, handle, dirty: editor.dirty } satisfies Saved, 'doc')
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

const discard = (editor: Editor) => !editor.dirty || confirm(`Discard unsaved changes to ${editor.file.name}?`)

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
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.satz'
  input.onchange = () => input.files?.[0] && load(input.files[0], null)
  input.click()
}

/** Starts over with the sample document. */
export function start(editor: Editor) {
  if (!discard(editor)) return
  const fresh = new Engine()
  editor.load(fresh.save())
  fresh.free()
}
