# Milestone 1: poster and booklet, engine → canvas → PDF with bleed

Done when: on the live GitHub Pages build, a user creates an A2 poster and an
8-page A5 booklet that use every M1 feature (master pages, threaded text, effects,
CMYK + spot color), and exports both as PDF with TrimBox, BleedBox and crop marks
that match the canvas.

Out of scope for M1: PDF/X, components, realtime collaboration, Figma import,
font helper, i18n.

## Settled design

- Engine (Rust → WASM) owns the Loro doc, layout and undo. React sends commands and
  reads snapshots.
- Engine emits one binary display list into WASM memory. A TS renderer reads it
  zero-copy and draws with CanvasKit (Skia, WebGL2), caching one SkPicture per
  item. The PDF writer (`krilla`) consumes the same display list.
- Text: `harfrust` shaping, `hypher` hyphenation, own Knuth-Plass. CanvasKit draws
  glyph IDs from the same font bytes.
- Color: document is RGB or CMYK + spot colors. Screen preview via `moxcms` with a
  bundled FOGRA51 profile built from the ICC registry data (`engine/icc/build`); the
  ECI's PSO Coated v3 may not be redistributed.
- UI: Figma UI3 layout, behavior and keybinds (Ctrl for Cmd), dark pro look,
  English, units mm by default (switchable), type sizes in pt.
- Browsers: Chromium + Firefox; Safari best effort.
- License: ISC.

## Breadth

### 13. Images
- Place PNG/JPEG; fit/fill, crop by moving content; effective ppi.
- Per document, chosen at creation: embed or link. Linking uses File System
  Access (Chromium); Firefox shows the option disabled with a hint. Missing
  links reported by preflight with relink.
- The `.satz` file and the autosave carry embedded images.

### 15. Preflight
- Missing links and images below 300 ppi, with images (13).
- Export warns but does not block.

### 16. Ship
- Example A2 poster and 8-page A5 booklet in the repo, exported by a test and
  covered by the canvas-vs-PDF check.
- README with screenshot and live link.

## Later

- Font picker as in Figma (family and style), with local fonts (Local Font Access)
  in the list; a font in text styles. Today fonts are added by upload, and Local
  Font Access only finds missing ones.
- WOFF2 fonts.
- Colour profiles: choose or upload a CMYK profile (e.g. PSO Coated v3) instead of the
  built-in FOGRA51.
- Auto layout: drag layers into and out of an auto layout frame on the canvas.
- Arrow keys in the 3×3 auto layout alignment grid.
- Undo and redo (Ctrl+Z) while the local variables dialog is open.
- Variables and collections as nested Loro maps instead of whole entries, so
  concurrent edits of one variable merge.
- Performance of `settle` and `lay_out`, which walk the whole tree after every
  command.
- Masters based on other masters; overriding layers nested in a master's groups
  and frames.
- Threading: click an in-port to thread a frame in before another, as InDesign does.
