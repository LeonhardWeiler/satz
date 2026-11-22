import { createHash } from 'node:crypto'
import { readdirSync, readFileSync } from 'node:fs'
import { defineConfig } from 'vite-plus'
import react from '@vitejs/plugin-react'

const base = '/'

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
      transformIndexHtml: (html) => [
        {
          tag: 'meta',
          attrs: {
            'http-equiv': 'Content-Security-Policy',
            content: `default-src 'self'; script-src 'self' 'wasm-unsafe-eval' ${[...html.matchAll(/<script>([^]*?)<\/script>/g)].map(([, s = '']) => `'sha256-${createHash('sha256').update(s).digest('base64')}'`).join(' ')}; img-src 'self' blob: data:; style-src 'self' 'unsafe-inline'`,
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
        if (!/^Module "node:fs" has been externalized .*\/canvaskit\.js"/.test(log.message)) handler(level, log)
      },
    },
  },
  server: { fs: { allow: ['..'] } },
  test: { include: ['src/**/*.test.ts', 'lint/*.test.ts'] },
  lint: {
    ignorePatterns: ['dist', 'canvaskit', 'src/engine', 'src/engine-io', 'src/types.ts', 'test-results', 'playwright-report'],
    plugins: ['eslint', 'oxc', 'react', 'unicorn', 'typescript'],
    jsPlugins: ['./lint/index.ts'],
    categories: { correctness: 'error', suspicious: 'error', perf: 'error' },
    options: { typeAware: true },
    env: { browser: true },
    rules: {
      'react/jsx-key': 'off',
      'eslint/no-control-regex': 'off',
      'eslint/no-shadow': 'off',
      'eslint/no-await-in-loop': 'off',
      'eslint/no-underscore-dangle': 'off',
      'typescript/no-this-alias': 'off',
      'oxc/no-map-spread': 'off',
      'react/no-object-type-as-default-prop': 'off',
      'react/jsx-no-constructed-context-values': 'off',
      'react/react-in-jsx-scope': 'off',
      ...Object.fromEntries(
        ['prefer-add-event-listener', 'prefer-string-starts-ends-with', 'prefer-set-has', 'prefer-array-find', 'no-new-array', 'no-array-reverse', 'no-array-sort', 'consistent-function-scoping'].map((r) => [`unicorn/${r}`, 'off']),
      ),
      'react/rules-of-hooks': 'error',
      'react/exhaustive-deps': 'warn',
      ...Object.fromEntries(
        ['capitalized-calls', 'error-boundaries', 'exhaustive-effect-dependencies', 'globals', 'immutability', 'incompatible-library', 'memo-dependencies', 'no-array-index-key', 'no-unstable-nested-components', 'preserve-manual-memoization', 'purity', 'refs', 'set-state-in-effect', 'set-state-in-render', 'static-components', 'unsupported-syntax', 'use-memo'].map((r) => [`react/${r}`, 'error']),
      ),
      'oxc/no-accumulating-spread': 'error',
      'typescript/no-extraneous-class': 'error',
      'eslint/no-empty-pattern': 'error',
      'eslint/no-new': 'error',
      'eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' }],
      'eslint/no-unsafe-optional-chaining': 'error',
      'eslint/no-unneeded-ternary': 'error',
      'unicorn/no-useless-spread': 'error',
      'unicorn/no-useless-fallback-in-spread': 'error',
      'unicorn/require-module-specifiers': 'error',
      'typescript/array-type': 'error',
      'typescript/consistent-indexed-object-style': 'error',
      'typescript/consistent-type-definitions': ['error', 'type'],
      'typescript/no-dynamic-delete': 'error',
      'typescript/no-unnecessary-condition': 'error',
      'typescript/no-unsafe-type-assertion': 'off',
      'typescript/no-misused-spread': 'off',
      'typescript/consistent-return': 'off',
      'typescript/restrict-template-expressions': ['error', { allowNumber: true }],
      'typescript/prefer-nullish-coalescing': ['error', { ignorePrimitives: { boolean: true } }],
      ...Object.fromEntries(
        ['no-floating-promises', 'no-misused-promises', 'no-unnecessary-type-assertion', 'use-unknown-in-catch-callback-variable', 'no-duplicate-type-constituents', 'prefer-promise-reject-errors', 'restrict-plus-operands', 'prefer-optional-chain', 'prefer-includes', 'prefer-string-starts-ends-with', 'prefer-regexp-exec', 'no-unnecessary-type-parameters', 'unbound-method', 'no-unsafe-argument', 'no-unsafe-assignment', 'no-unsafe-call', 'no-unsafe-member-access', 'no-unsafe-return'].map((r) => [`typescript/${r}`, 'error']),
      ),
    },
    overrides: [
      { files: ['src/**'], rules: { 'eslint/no-restricted-properties': ['error', { object: 'WebAssembly', property: 'RuntimeError', message: 'A trap reaches stop() through Engine.onPanic.' }, { object: 'window', property: 'localStorage', message: 'Stored data goes through load and store in settings.ts.' }] } },
      { files: ['src/**'], excludeFiles: ['src/settings.ts'], rules: { 'eslint/no-restricted-globals': ['error', { name: 'localStorage', message: 'Stored data goes through load and store in settings.ts.' }] } },
      { files: ['src/ContextMenu.tsx'], rules: { 'react/no-array-index-key': 'off', 'typescript/no-base-to-string': 'off' } },
      { files: ['e2e/**', 'shots/**', 'src/**/*.test.ts', 'lint/**/*.test.ts'], excludeFiles: ['**/playwright.config.ts'], rules: { 'satz/no-test-timing': 'error' } },
      { files: ['src/**'], excludeFiles: ['src/**/*.test.ts', 'src/ColorPicker.tsx', 'src/Canvas.tsx'], rules: { 'satz/no-hex-color': 'error' } },
      { files: ['src/**'], rules: { 'satz/no-cached-rejection': 'error', 'satz/no-shortcut-in-title': 'error', 'satz/way-out': 'error' } },
      { files: ['src/**'], excludeFiles: ['src/**/*.test.ts', 'src/SettingsDialog.tsx', 'src/Shortcuts.tsx', 'src/Properties.tsx'], rules: { 'satz/no-unit-text': 'error' } },
      { files: ['sw.js'], env: { serviceworker: true }, globals: { CACHE: 'readonly', FILES: 'readonly' }, rules: Object.fromEntries(['argument', 'assignment', 'call', 'member-access', 'return'].map((r) => [`typescript/no-unsafe-${r}`, 'off'])) },
    ],
  },
})
