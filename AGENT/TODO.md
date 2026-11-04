# Milestone 1: poster and booklet, engine → canvas → PDF with bleed

Done when: on the live GitHub Pages build, a user creates an A2 poster and an
8-page A5 booklet that use every M1 feature (master pages, threaded text, effects,
CMYK + spot color), and exports both as PDF with TrimBox, BleedBox and crop marks
that match the canvas.

Out of scope for M1: components, realtime collaboration, Figma import,
font helper, i18n.

## Settled design

- Engine (Rust → WASM) owns the Loro doc, layout and undo. React sends commands and
  reads snapshots.
- Engine emits one binary display list into WASM memory. A TS renderer reads it
  zero-copy and draws with CanvasKit (Skia, WebGL2), caching one SkPicture per
  item. The PDF writer (`krilla`) consumes the same display list.
- Text: `harfrust` shaping, `hypher` hyphenation, own Knuth-Plass. CanvasKit draws
  glyph IDs from the same font bytes.
- Color: document is RGB or CMYK + spot colors. Screen preview via `moxcms`, with black
  point compensation, and a bundled FOGRA51 profile built from the ICC registry data (`engine/icc/build`); the
  ECI's PSO Coated v3 may not be redistributed.
- UI: design G, fixed: a top bar, layers and
  swatches left, properties or the preflight mode right, mm rulers; no floating
  panels, no bottom bar. Pages in the overview on "." over the canvas, which always
  shows one spread. Figma keybinds (Ctrl for Cmd), dark look, English, units mm by default
  (switchable), type sizes in pt.
- Browsers: Chromium + Firefox; Safari best effort.
- License: ISC.

## Now

IDs name finding cards in `AGENT/project-health-report.html`; read the card for cause and fix.

- [ ] TEST-1, blocked: Playwright's Firefox gets no WebGL 2 context headless or under Xvfb.

## Performance

Profile and numbers in `AGENT/performance.md`. Measure before and after each item.

- [ ] Browser benchmark: Playwright with `performance.now()` around `apply`, `snapshot` +
  `JSON.parse`, `index()` and `paint` on 1, 8 and 32 pages.
- [ ] Keep the `perf` setup as an ignored bench (`PHASE=… cargo test prof`) with frame pointers.
- [ ] `opt-level = 3` (or per package for harfrust, tiny-skia, linesweeper, moxcms, loro); compare
  WASM size.
- [ ] WASM with `+simd128,+bulk-memory`; tiny-skia's `simd` is scalar without `simd128`.
- [ ] Cache decoded display lists in `renderer.ts` by `(id, listVersion)` instead of `.slice()` +
  `decode()` per paint.
- [ ] Cache the bleed path union in `Renderer.draw` by the sheets instead of `Path.MakeFromOp` per
  frame.
- [ ] Build `palette()` once per `finish` for `flow_all` and `build_snapshot`.
- [ ] `use_profile` compares the 240 KB ICC bytes on every commit; compare a hash.
- [ ] Cache `to_rgb` per CMYK value in the thread's `Cms`.
- [ ] `Doc::save` reads image hashes through serde_json (26 % of save); read the `LoroValue`.
- [ ] `hit`: reject by rotated bounds first, collect sibling masks once per level.
- [ ] Palette and variable changes still lay out every root in `Doc::finish`.
- [ ] Cache `kind` per `TreeID`, invalidated on structural changes.
- [ ] Deserialize node meta straight from `LoroValue` in `snap`, `layout`, `flow`, `snapshot_of`.
- [ ] Shapers and metrics per font id in a `thread_local` instead of per `text::rows` call.
- [ ] Cache hyphenation points per word and language.
- [ ] Cache Knuth-Plass breaks per paragraph by (shaped paragraph, column width, wrap insets).
- [ ] `text::draw`: `run_text` and ranges only for the PDF, paint and font class ids per span,
  `Vec::with_capacity`, no `str::replace` per run.
- [ ] Cache `Style::shape` outlines per (shape, size).
- [ ] Snapshot JSON: short number formatter (`ryu`, 2 decimals) and ids without escaping, or binary.
- [ ] Coalesce drag `SetFrame` to 1 per animation frame.
- [ ] Export worker: send Loro updates since its version vector instead of `save()` + `load()`.
- [ ] Autosave appends Loro updates to IndexedDB and compacts now and then.
- [ ] Optimistic typing and drag without the engine (transform the cached `SkPicture`, not when
  text wraps around the item).
- [ ] Split `watch()` so hovers notify only the overlay; virtualize the layers tree.
- [ ] Snapping: sort the lines once per drag, binary search in `snap`, `nearest`, `spacings`.
- [ ] Canvas: pan and zoom on the last bitmap, keep overlay SVG elements, group end index in the
  display list, cache blur and shadow layers per item and zoom, share the picture cache with the
  overview, render neighbours and thumbnails in idle time, tiles per zoom level.
- [ ] Raster: layer pixmaps and clip masks bounded to the item, no `out` layer without shadow,
  blur only alpha, integer offsets for lowp, downsample large sigmas, cache rasterized items by
  hash, faster deflate.
- [ ] Inks per dirty rect, 1 pass with an N-channel blitter, separation per distinct colour.
- [ ] PDF: cache encoded image streams by hash, subset fonts and encode images in workers.
- [ ] Startup: `wasm-opt -O3`, `talc` or `lol_alloc`, move photo, FOGRA51 and fonts out of the
  WASM, cache the compiled module, store the snapshot next to the Loro bytes, a smaller CanvasKit.
- [ ] Large rebuilds: `u32` node handles, memoized queries replacing `sets`/`sheets`/`flows`/
  `kept_sheets`/`drawn`, a typed arena from Loro events, WASM threads, a render worker, inks on
  WebGPU.

## Later

- [ ] Threading through the in-port (explain to the user before building it): in InDesign each
  text frame has an in-port at its top left and an out-port at its bottom right. Clicking the
  in-port of a frame that is already in a thread and then another frame puts that frame *before*
  it, so the story flows through the new frame first. Satz can only append through `next`.
- [ ] Figma import from a file or an account.
- [ ] Start page listing all projects, as in Figma.
- [ ] Table of contents.
- [ ] Date variable, like the page number.
- [ ] Conditional text with variables.
- [ ] Tables.
- [ ] Anchored objects.
- [ ] Linked images: per document, chosen at creation, embed or link. Linking uses File System
  Access (Chromium) with handles in IndexedDB, so links need a click per session and are missing
  on other computers; Firefox shows the option disabled with a hint. Preflight reports missing
  links with relink.
- [ ] Variable font axes as in Figma. krilla 0.8.2 exports them (`Font::new_variable` instances
  the subset) and harfrust shapes them; CanvasKit 0.42 has no variation binding, so the canvas
  draws such glyphs as paths from skrifa at the font's location, cached per font, location and
  glyph.
- [ ] Place SVG and PDF as vectors, kept as vectors in the exported PDF.
- [ ] Version history from the Loro history: browse and restore earlier states.
- [ ] GREP styles.
- [ ] Colour wheel in the picker for harmonies (analogous, complementary, …).
- [ ] AI connection for the common providers, as in Figma.
- [ ] With a server: work together on one project through a link or code.
