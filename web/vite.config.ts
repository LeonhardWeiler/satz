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
  ],
  server: { fs: { allow: ['..'] } },
})
