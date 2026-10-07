# Performance

Native release bench (`engine/src/doc/bench.rs`), median ms, 2026-10-07:

| pages | set_frame | keystroke | pdf  |
| ----- | --------- | --------- | ---- |
| 1     | 0.24      | 0.58      | 47   |
| 8     | 0.59      | 4.93      | 284  |
| 32    | 2.11      | 10.91     | 1075 |

Inks: 86 ms at 36 ppi, 289 ms at 72 ppi. Per command and per drag step the work
grows with the document, not with the change.

`perf` on 32 pages (share of the phase, inclusive):

- set_frame: `flow_all` 60 % (its cache check rereads Loro: `text_frame_of` 35 %,
  spans 20 %), `kind` 17 %, `lay_out` 14 %, `build_snapshot` 12 %,
  `serde_json::to_value` 12 %, `settle` 10 %, `is_node_unexist` 7 %.
- keystroke: `text::set` 59 % (shaping 28 %, Knuth-Plass 9 %, hyphenation 6 %),
  `build_snapshot` 22 % (every page of the story is built again).
- render: `text::draw` 53 % (allocations 16 %, `String` drops 8 %,
  `str::replace` 5 %, `Paint` eq 3 %), `Style::shape` 8 %, encode 5 %.
- snapshot JSON: string escaping 22 %, memmove 20 %, float formatting 15 %.
- pdf: rasterizing 95 % (deflate 33 %, box blur 27 %, fills 21 %).
- inks and png: highp `draw_pixmap` 54–68 %, box blur 13–17 %.
- save: Loro export 59 % (lz4 19 %), the image scan through serde_json 26 %.
- load: `finish` 82 %.

## Measure first

- [ ] Browser benchmark: Playwright with `performance.now()` around `apply`,
  `snapshot` + `JSON.parse`, `index()` and `paint`, on 1, 8 and 32 pages. Native
  numbers hide the WASM and JS side.
- [ ] Keep the `perf` setup as an ignored bench (`PHASE=… cargo test prof`) with
  frame pointers, so that each item below can be measured before and after.

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
- [ ] Build `palette()` once per `finish` and pass it to `flow_all` and
  `build_snapshot`; it deserializes 4 lists through serde_json each time.
- [ ] `use_profile` compares the 240 KB ICC bytes on every commit; compare a hash or
  the Loro value's pointer.
- [ ] Cache `to_rgb` per CMYK value (a small map in the thread's `Cms`); moxcms runs
  for every CMYK paint of every render.
- [ ] `Doc::save` reads image hashes from fills through serde_json (26 % of save);
  read the `LoroValue` directly.
- [ ] `hit` builds the outline of every shape under the pointer path and checks the
  masks of all later siblings per node; reject by the rotated bounds first and
  collect the masks once per level.

## Commit path

- [ ] `flow`'s cache check rebuilds text, spans, frames and `text_frame_of` from Loro
  for every story (52 % of a drag step); keep the inputs dirty-flagged from the
  `changed` set and skip the stories it does not reach.
- [ ] `Doc::finish` settles and lays out only the roots that the `changed` set
  reaches; palette and variable changes still walk everything.
- [ ] Cache `kind` per `TreeID`, invalidated on structural changes; `Doc::node`
  walks to the root and reads `kind` as a string each time (17 %).
- [ ] Deserialize node meta straight from `LoroValue` instead of
  `get_deep_value()` → `serde_json::Value` → struct in `snap`, `layout`, `flow` and
  `snapshot_of`.
- [ ] Keep shapers and metrics per font id in a `thread_local` instead of building
  `ShaperData`, shapers and metrics on every `text::rows` call.
- [ ] Cache shaped glyphs per paragraph by (text, spans, font ids); a keystroke
  shapes 1 paragraph.
- [ ] Cache hyphenation points per word and language; `syllables` runs hypher on
  every word of the story.
- [ ] Cache Knuth-Plass breaks per paragraph by (shaped paragraph, column width,
  wrap insets).
- [ ] Threading stops early: when frame n+1 starts at the same byte offset and only
  bytes before it changed, keep the following frames with the offset shifted.
- [ ] Keep a story's snapshot pages whose placed lines did not change instead of
  building every page of the story when its `Rc` changes.

## Render path

- [ ] Compute `run_text` and the ranges in `text::draw` only for the PDF; the canvas
  never reads them, yet each paint clones them.
- [ ] Give each span a paint and font class id once per layout, so `chunk_by`
  compares 2 integers instead of `Vec<Paint>`.
- [ ] `Vec::with_capacity` for glyphs and runs in `text::draw`, and no
  `str::replace` per run.
- [ ] Cache `Style::shape` outlines per (shape, size); they are built per render
  and per hit.

## Engine ↔ JS

- [ ] Versioned snapshot per page: JS fetches only pages whose version changed and
  keeps the objects of the others, so React selectors rerender less and
  `JSON.parse` + `index()` shrink to the change.
- [ ] Until then, structural sharing after `JSON.parse`: reuse the old object where
  the new one is deep equal, so selectors keep their identity.
- [ ] Snapshot JSON: write numbers with a short formatter (`ryu`, 2 decimals for
  points) and ids without escaping; or a binary snapshot like the display list.
- [ ] Coalesce drag `SetFrame` calls to 1 per animation frame.
- [ ] Send Loro updates since the worker's version vector
  (`ExportMode::updates`) to the export worker and `import` them, instead of
  `save()` + `load()` of the whole document.
- [ ] Autosave appends Loro updates to IndexedDB and compacts to a snapshot now and
  then, instead of a full `save()` a second after each change.
- [ ] Optimistic typing: draw the keystroke with the old layout and the glyph put in
  locally, the real layout one frame later.
- [ ] Drag without the engine: transform the item's cached `SkPicture` while
  dragging and send `SetFrame` on release; not when text wraps around the item.

## Web UI

- [ ] Split `watch()`: pointer hovers notify only the overlay, not every
  `useEditor` selector.
- [ ] Virtualize the layers tree; it renders every node of the page.
- [ ] Snapping and smart guides: sort the lines once per drag start and binary
  search in `snap`, `nearest` and `spacings` instead of values × lines per move;
  compute the distance labels at most once per frame.

## Canvas

- [ ] Pan without redraw: draw the scene with a margin around the viewport and
  translate the image while panning, redraw at the edge or when the pan stops.
- [ ] Zoom gestures scale the last bitmap and redraw sharp once the gesture ends.
- [ ] Keep the overlay SVG elements and set attributes instead of rebuilding
  `innerHTML` on every pointer move.
- [ ] Store each group's end index in the display list instead of the `close()`
  scan while drawing.
- [ ] Cache blur and shadow layers as images per item and zoom; `saveLayer` with an
  image filter runs every frame now. Reuse `Paint` and `ImageFilter` objects.
- [ ] Share the picture cache between the overview and the main canvas.
- [ ] Render the neighbouring pages' display lists and the overview thumbnails in
  idle time.
- [ ] Tiles cached per zoom level; redraw only tiles that a changed item's bounds
  cross.

## Raster (PDF, PNG, inks)

- [ ] Bound layer pixmaps to the item's bounds plus the blur margin instead of the
  whole page.
- [ ] Skip the `out` layer when an item has no shadow.
- [ ] Blur only the alpha channel for shadows and tint afterwards: 1 channel
  instead of 4.
- [ ] Composite layers at integer offsets so tiny-skia stays on the lowp pipeline,
  or a hand-written same-size source-over with opacity.
- [ ] Blur large sigmas on a downsampled pixmap and scale back.
- [ ] Bound clip masks to the clip's bounds.
- [ ] Cache rasterized items across exports and inks by a hash of the item, not a
  serde_json key.
- [ ] Deflate raster images at a lower level or with a faster deflater (33 % of
  the PDF).

## Export and preflight

- [ ] Inks per dirty rect: rasterize only the bounds of changed items again.
- [ ] 1 raster pass for all inks with an N-channel blitter instead of 1 pass per 3
  inks.
- [ ] Separate inks per distinct colour or through the LUT instead of moxcms per
  pixel.
- [ ] Cache encoded image streams for the PDF by content hash across exports.
- [ ] Subset fonts and encode images in parallel workers, krilla only assembles.

## Startup

- [ ] `wasm-opt -O3`; try `talc` or `lol_alloc` instead of dlmalloc.
- [ ] The WASM is 8.3 MB; move the examples' photo (480 KB), FOGRA51 (240 KB) and
  the fonts out of the binary and fetch them when needed.
- [ ] Cache the compiled WASM module in IndexedDB.
- [ ] Load a saved document without layout: store the snapshot next to the Loro
  bytes, so `finish` (82 % of load) runs in idle time.
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
