import init, { Engine } from './engine/engine'

const engine = init().then(() => new Engine())

onmessage = async ({ data: { doc, fonts } }: MessageEvent<{ doc: Uint8Array; fonts: Uint8Array[] }>) => {
  try {
    const e = await engine
    for (const bytes of fonts) e.addFont(bytes)
    e.load(doc)
    const pdf = e.pdf()
    postMessage({ pdf }, { transfer: [pdf.buffer] })
  } catch (e) {
    postMessage({ error: (e as Error).message })
  }
}
