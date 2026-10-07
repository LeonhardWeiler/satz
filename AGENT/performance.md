# Performance

Native release bench (`engine/src/doc/bench.rs`), median ms, 2026-10-07:

| pages | set_frame | keystroke | pdf  |
| ----- | --------- | --------- | ---- |
| 1     | 0.24      | 0.58      | 47   |
| 8     | 0.59      | 4.93      | 284  |
| 32    | 2.11      | 10.91     | 1075 |

Inks: 86 ms at 36 ppi, 289 ms at 72 ppi. Per command and per drag step the work
grows with the document, not with the change.

## Measure first

- [ ] Browser benchmark: Playwright with `performance.now()` around `apply`,
  `snapshot` + `JSON.parse`, `index()` and `paint`, on 1, 8 and 32 pages. Native
  numbers hide the WASM and JS side.

## Cheap wins

- [ ] `opt-level = 3` for the release build, or per package for harfrust,
  tiny-skia, linesweeper, moxcms and loro; compare WASM size.
- [ ] Build WASM with `+simd128,+bulk-memory`; tiny-skia's `simd` feature falls back
  to scalar code without `simd128`.
- [ ] Cache `Engine::plate` by page and version like `drawn()`; in CMYK every pan or
  zoom frame renders, recolors and encodes 2 plates again.
- [ ] Cache decoded display lists in `renderer.ts` by `(id, listVersion)` instead of
  `.slice()` + `decode()` on every paint.
- [ ] `drawn()` renders every requested page after each snapshot only to compare;
  `Rc::ptr_eq` on the kept `Rc<Page>` tells that it is unchanged.
- [ ] Cache the bleed path union in `Renderer.draw` by the sheets instead of
  `Path.MakeFromOp` per frame.

## Commit path

- [ ] `Doc::finish` settles and lays out only the roots that the `changed` set
  reaches; palette and variable changes still walk everything. Same for
  `flow_all()`.
- [ ] Keep shapers and metrics per font id in a `thread_local` instead of building
  `ShaperData`, shapers and metrics on every `text::rows` call.
- [ ] Cache shaped glyphs per paragraph by (text, spans, font ids); a keystroke
  shapes 1 paragraph.
- [ ] Cache Knuth-Plass breaks per paragraph by (shaped paragraph, column width,
  wrap insets).
- [ ] Threading stops early: when frame n+1 starts at the same byte offset and only
  bytes before it changed, keep the following frames with the offset shifted.
- [ ] Deserialize node meta straight from `LoroValue` instead of
  `get_deep_value()` → `serde_json::Value` → struct in `snap`, `layout`, `flow` and
  `snapshot_of`.
- [ ] Cache `kind` per `TreeID`, invalidated on structural changes; `Doc::node`
  walks to the root and reads `kind` as a string each time.

## Engine ↔ JS

- [ ] Versioned snapshot per page: JS fetches only pages whose version changed and
  keeps the objects of the others, so React selectors rerender less and
  `JSON.parse` + `index()` shrink to the change.
- [ ] Send Loro updates since the worker's version vector
  (`ExportMode::updates`) to the export worker and `import` them, instead of
  `save()` + `load()` of the whole document.
- [ ] Optimistic typing: draw the keystroke with the old layout and the glyph put in
  locally, the real layout one frame later.
- [ ] Drag without the engine: transform the item's cached `SkPicture` while
  dragging and send `SetFrame` at most once per frame or on release; not when text
  wraps around the item.

## Canvas

- [ ] Pan without redraw: draw the scene with a margin around the viewport and
  translate the image while panning, redraw at the edge or when the pan stops.
- [ ] Zoom gestures scale the last bitmap and redraw sharp once the gesture ends.
- [ ] Keep the overlay SVG elements and set attributes instead of rebuilding
  `innerHTML` on every pointer move.
- [ ] Render the neighbouring pages' display lists and the overview thumbnails in
  idle time.
- [ ] Tiles cached per zoom level; redraw only tiles that a changed item's bounds
  cross.

## Export and preflight

- [ ] Inks per dirty rect: rasterize only the bounds of changed items again.
- [ ] Separate inks per distinct colour or through the LUT instead of moxcms per
  pixel.
- [ ] Cache encoded image streams for the PDF by content hash across exports.
- [ ] Profile the 33 ms per PDF page; subset fonts and encode images in parallel
  workers, krilla only assembles.

## Startup

- [ ] `wasm-opt -O3`; try `talc` or `lol_alloc` instead of dlmalloc.
- [ ] Cache the compiled WASM module in IndexedDB; load FOGRA51 and bundled fonts
  lazily if they are in the binary.
- [ ] A smaller CanvasKit build without Skottie and Paragraph.

## Large rebuilds

- [ ] Intern node ids as `u32` handles inside the engine and across the boundary
  instead of `String` in maps, sets and results.
- [ ] Memoized queries with dependency tracking for `flow`, `layout`, `snap` and
  `render`, replacing `sets`, `sheets`, `flows`, `kept_sheets` and `drawn` with one
  mechanism.
- [ ] A flat typed arena of the document kept from Loro's events; reads become field
  accesses, Loro stays for persistence, undo and sync.
- [ ] WASM threads (`SharedArrayBuffer`, `wasm-bindgen-rayon`) for rasterizing and
  inks; COOP/COEP through the service worker.
- [ ] Render worker: CanvasKit on an `OffscreenCanvas`, the main thread only runs
  React and input.
- [ ] Inks on the GPU (WebGPU in the worker): coverage, TAC and gamut as a shader and
  a reduction.
