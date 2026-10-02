# Milestone 1: poster and booklet, engine → canvas → PDF with bleed

Done when: on the live GitHub Pages build, a user creates an A2 poster and an
8-page A5 booklet that use every M1 feature (master pages, threaded text, effects,
CMYK + spot color), and exports both as PDF with TrimBox, BleedBox and crop marks
that match the canvas.

Out of scope for M1: components, realtime collaboration, Figma import,
font helper, i18n.

## UI design G (branch ui-g, spec AGENT/ui-ux/g-register.html)

Built: everything in `AGENT/ui-g-inventory.md`.
Open:

- Commit 5b5a630 does not build alone (the Pages.tsx deletion landed there).

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
- UI: design G (`AGENT/ui-ux/g-register.html`), fixed: a top bar, layers and
  swatches left, properties or the preflight mode right, mm rulers; no floating
  panels, no bottom bar. Pages in the overview on "." over the canvas, which always
  shows one spread. Figma keybinds (Ctrl for Cmd), dark look, English, units mm by default
  (switchable), type sizes in pt.
- Browsers: Chromium + Firefox; Safari best effort.
- License: ISC.

## Later

- Linked images: per document, chosen at creation, embed or link. Linking uses
  File System Access (Chromium) with handles in IndexedDB, so links need a click
  per session and are missing on other computers; Firefox shows the option
  disabled with a hint. Preflight reports missing links with relink.
- Type as in Figma:
  vertical trim to cap height and baseline, axes of variable fonts, a glyph panel
  with search for all characters of a font.
- Colour profiles: choose or upload a CMYK profile (e.g. PSO Coated v3) instead of the
  built-in FOGRA51.
- Auto layout: drag layers into and out of an auto layout frame on the canvas.
- Performance of `settle` and `lay_out`, which walk the whole tree after every
  command.
- Threading: click an in-port to thread a frame in before another, as InDesign does. (versteh ich nicht ganz bitte erklären bevor es umgesetzt wird)
- Rotation: a turned multi-selection resized
  in its own frame, ports on turned text frames.
- Text wrap around layers, with an offset, for text frames under images and shapes.
- Place SVG and PDF as vectors, kept as vectors in the exported PDF.
- Package: one ZIP with the document, its fonts and images for the printer.
- Version history from the Loro history: browse and restore earlier states.
- Overprint for fills and strokes, shown in the separations preview and written to
  the PDF.
- Spell check from Hunspell word lists in the engine, marked in the text.
