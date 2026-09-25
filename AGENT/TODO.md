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

### 10. Rich text
- Character and paragraph attributes on Loro rich text; text styles.
- Hyphenation with `hypher` in the Knuth-Plass line breaker, per paragraph on/off
  and language.
- Text frame model: insets, columns + gutter, vertical alignment, baseline grid.
- Figma resize modes (auto width, auto height, fixed).
- In-frame editing: engine draws caret and selection, hidden textarea for
  keyboard and IME.

### 11. Pages, masters, threading
- Multiple pages, pages panel, master pages applied per page.
- Frame threading across frames and pages; only fixed frames thread, the last
  frame of a chain may be auto height. Overset detection.

### 12. Fonts
- Upload, bundled font, Local Font Access (Chromium).
- Fonts referenced by name + hash; missing fonts fall back to the bundled font,
  highlighted pink, reported by preflight.

### 13. Images
- Place PNG/JPEG; fit/fill, crop by moving content; effective ppi.
- Per document, chosen at creation: embed or link. Linking uses File System
  Access (Chromium); Firefox shows the option disabled with a hint. Missing
  links reported by preflight with relink.

### 14. Persistence
- Autosave Loro snapshot to IndexedDB.
- `.satz` project file: Loro snapshot + embedded images (when embedding).

### 15. Preflight
- Overset text, missing fonts, missing links, images below 300 ppi, objects at the
  trim edge that stop short of the bleed, RGB content in a CMYK document.
- Panel lists issues; click selects the item. Export warns but does not block.

### 16. Ship
- Example A2 poster and 8-page A5 booklet in the repo, exported by a test and
  covered by the canvas-vs-PDF check.
- README with screenshot and live link.

## Later

- Colour profiles: choose or upload a CMYK profile (e.g. PSO Coated v3) instead of the
  built-in FOGRA51.
