import { createHash } from 'node:crypto'
import { readdirSync, readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const base = '/satz/'

export default defineConfig({
  base,
  plugins: [
    react(),
    {
      name: 'preload-wasm',
      transformIndexHtml: (_, { bundle }) =>
        Object.keys(bundle ?? {})
          .filter((f) => /\/(engine_bg|canvaskit)-[\w-]+\.wasm$/.test(f))
          .map((f) => ({
            tag: 'link',
            attrs: { rel: 'preload', as: 'fetch', type: 'application/wasm', crossorigin: true, href: base + f },
            injectTo: 'head',
          })),
    },
    {
      name: 'content-security-policy',
      apply: 'build',
      transformIndexHtml: () => [
        {
          tag: 'meta',
          attrs: {
            'http-equiv': 'Content-Security-Policy',
            content: "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; img-src 'self' blob: data:; style-src 'self' 'unsafe-inline'",
          },
          injectTo: 'head-prepend',
        },
      ],
    },
    {
      name: 'service-worker',
      apply: 'build',
      generateBundle(_, bundle) {
        const pub = readdirSync(new URL('public', import.meta.url))
        const files = ['', ...pub, ...Object.keys(bundle)].map((f) => base + f)
        const hash = createHash('sha256').update(files.join())
        for (const f of ['index.html', ...pub.map((f) => `public/${f}`)]) hash.update(readFileSync(new URL(f, import.meta.url)))
        for (const f of Object.values(bundle)) hash.update(f.type === 'chunk' ? f.code : f.source)
        const cache = hash.digest('hex').slice(0, 16)
        const source = `const CACHE = 'satz-${cache}'\nconst FILES = ${JSON.stringify(files)}\n${readFileSync(new URL('sw.js', import.meta.url), 'utf8')}`
        this.emitFile({ type: 'asset', fileName: 'sw.js', source })
      },
    },
  ],
  build: {
    chunkSizeWarningLimit: 600,
    rolldownOptions: {
      onLog: (level, log, handler) => {
        if (!/^Module "(fs|path)" has been externalized .*\/canvaskit\.js"/.test(log.message)) handler(level, log)
      },
    },
  },
  server: { fs: { allow: ['..'] } },
})
