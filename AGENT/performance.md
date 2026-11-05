# Performance

Native release bench (`engine/src/doc/bench.rs`), median ms, 2026-11-05:

| pages | set_frame | keystroke | json | save |
| ----- | --------- | --------- | ---- | ---- |
| 1     | 0.46      | 0.66      | 0.14 | 0.95 |
| 8     | 0.49      | 1.08      | 1.05 | 1.88 |
| 32    | 0.56      | 1.36      | 4.21 | 5.44 |

PDF of 1 page 31 ms (1075 ms for 32 pages and 47 ms for 1 on 2026-10-07). Inks 15.6 ms at
36 ppi, 31.1 ms at 72 ppi (86 and 289 ms on 2026-10-07). `Doc::load` of 32 pages 121 ms,
`finish` 78 ms of it.

Browser (`PERF=1 PAGES=32 ./check e2e e2e/perf.spec.ts`), ms per step on 32 pages:

| step      | apply | snapshot | paint |
| --------- | ----- | -------- | ----- |
| dragStep  | 2.3   | 0.6      | 6.6   |
| keystroke | 4.1   | 0.7      | 6.9   |
| pan       | 0     | 0        | 5.0   |

Measured and not done:

- Optimistic typing and drag: at most 3 of ~10 ms per step, and it needs a second copy of
  position and text logic in TS.
- Virtualized or memoized layers tree: 0.24 ms per drag step.
- Snapping with sorted lines and binary search: snapping takes 0.4 ms per step.
- Canvas: overlay SVG 0.05 ms, pan paints in 5 ms; tiles, idle rendering, blur caches and
  a shared picture cache are rewrites without a bottleneck.
- Raster: blur of alpha only, integer offsets, downsampled sigmas and a raster cache by
  hash; the heavy document has no shadows and rasterizing runs in the worker.
- Inks per dirty rect and an N-channel blitter: inks run in the worker in 31 ms.
- PDF fonts: krilla always subsets. Image encoding in workers: images are encoded once.
- `wasm-opt -O3` and `talc`: no change beyond noise, +57 and +63 KB WASM.
- Font and FOGRA51 out of the WASM: 0.5 of 9.2 MB, both needed at start or for CMYK.
- Cached compiled module: the browser caches the streamed WASM compile.
- Stored snapshot: load spends its time in `finish`, which the snapshot does not replace.
- Smaller CanvasKit: needs an own emscripten build.
- Large rebuilds (`u32` handles, memoized queries, typed arena, threads, render worker,
  WebGPU inks): the largest step costs 4 ms in the engine.
- Earlier: `use_profile` hash, `kind` cache, snapshot JSON formatter, `run_text` only for
  the PDF.
