# Milestone 1: poster and booklet, engine → canvas → PDF with bleed

Done when: on the live GitHub Pages build, a user creates an A2 poster and an
8-page A5 booklet that use every M1 feature (master pages, threaded text, effects,
CMYK + spot color), and exports both as PDF with TrimBox, BleedBox and crop marks
that match the canvas.

Out of scope for M1: PDF/X, components, realtime collaboration, Figma import,
font helper, i18n.

## Bugs

Found by the review on 2026-09-25 (IDs from `AGENT/project-health-report.html`), causes checked in
the code, not reproduced in the browser.

### B16. Clicking a panel while drawing with the pen splits the path's undo step (BUG-9)
- Cause: `Editor.gesture` sends `beginUndoGroup`, which ends every open group, including the pen's.
- Fix: `gesture` calls `finishPen(false)` first, as undo already does.
- Test: e2e draws two anchors, clicks the properties panel, undoes once, the path is gone.

### B17. Colour picker stays put when the properties panel scrolls (UX-6)
- Cause: `Popover` re-places only on window resize and its own size change.
- Fix: also listen to `scroll` with capture in the same effect.

### B18. Space cannot activate focused buttons (A11Y-2)
- Cause: `Canvas.tsx` `onKey` prevents Space on keydown and keyup for any target that is not an
  input.
- Fix: take Space for panning only when the target is the canvas or the body.
- Test: e2e focuses a layer row button, presses Space, the layer is selected.

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

## Thin slice

### 1. Workspace scaffold
- Cargo workspace with crate `engine` (`cdylib` + `rlib`), built with
  `wasm-pack build --target web` into `web/src/engine`.
- `web/`: Vite + React + TypeScript + `canvaskit-wasm`, pnpm.
- One `check` entry point (`cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`, `tsc --noEmit`, `pnpm lint`) used by commits and CI.
- GitHub Actions: checks, deploy `web/dist` to GitHub Pages.

### 2. Minimal document and display list
- Loro doc: one page (trim size, bleed), a rectangle and a text frame, geometry in pt.
- Command API across the WASM boundary; snapshot for React.
- Binary display list format (paths, fills, glyph runs, images) with a Rust
  encoder and TS decoder, tested with a roundtrip.

### 3. First text
- Bundled open font, `harfrust` shaping, Knuth-Plass into one fixed frame.
- TDD against fixed font and text.

### 4. Canvas
- TS renderer draws the display list with CanvasKit; pan, zoom, HiDPI; trim and
  bleed guides.

### 5. First PDF
- `krilla`: page = trim + bleed, TrimBox, BleedBox, crop marks, embedded font subset.
- Playwright test: canvas screenshot vs. PDF rasterized with MuPDF, pixel diff
  with tolerance, in CI.
- Deploy; thin slice is live.

## Breadth

### 6. Object tree and editing
- Figma tree: groups, containers with clip content, z-order.
- Hit testing, selection, move/resize handles, Figma keybinds, undo/redo via Loro
  `UndoManager`.
- UI3 shell: floating toolbar, layers left, properties right, dark theme.

### 7. Shapes and paint
- Rectangle (corner radius), ellipse, line, arrow, polygon, star; pen tool. One
  path model.
- Solid fills, linear and radial gradients, strokes, opacity, blend modes.
- Drop shadow, blur, masks. PDF rasterizes shadow and blur at the document
  setting (default 300 ppi); everything else stays vector.

### 9. Variables and constraints
- Figma variables (color, number, modes) bound to properties.
- Constraints for children on container resize.
- Auto layout (direction, gap, padding, hug/fill/fixed).

### 10. Rich text
- Character and paragraph attributes on Loro rich text; text styles.
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
