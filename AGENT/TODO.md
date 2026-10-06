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

## Later

- Threading: click an in-port to thread a frame in before another, as InDesign does. (versteh ich nicht ganz bitte erklären bevor es umgesetzt wird)
