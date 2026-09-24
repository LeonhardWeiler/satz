# Milestone 1: poster and booklet, engine → canvas → PDF with bleed

Done when: on the live GitHub Pages build, a user creates an A2 poster and an
8-page A5 booklet that use every M1 feature (master pages, threaded text, effects,
CMYK + spot color), and exports both as PDF with TrimBox, BleedBox and crop marks
that match the canvas.

Out of scope for M1: PDF/X, components, realtime collaboration, Figma import,
font helper, i18n.

## Bugs

Found by hand on 2026-09-24 (notes in AGENT/bugs.md), causes checked in the code.

### B7. Colour picker is cut off at the window edge (bugs.md 15)
- Cause: the native `<input type="color">` popup is placed by the browser.
- Fix: own Figma-style colour picker popover (saturation/value area, hue, alpha, hex) placed with
  viewport collision (flip and shift). Item 8 needs it anyway for CMYK values and swatches, so
  build it there. The shape menu uses the same placement.

### B8. Text looks heavier in the exported PDF than on the canvas (bugs.md 3)
- Both draw the same glyph outlines (canvas-vs-PDF check passes). Likely rasterization: CanvasKit
  draws text without the contrast/gamma boost most PDF viewers apply.
- First step: compare canvas and PDF at 800 % in the same viewer setup. If outlines match, decide
  whether to thicken the preview (e.g. CanvasKit subpixel edging) or leave it; if not, fix the
  glyph run in `pdf.rs`.

### B9. Undo and tool switches while a pen path or drag is open (found while checking)
- Cause: Ctrl+Z inside the open pen undo group can remove the path node; the next `setPath`
  then throws. Undo during a drag mixes with the drag's undo group.
- Fix: `keys.ts` finishes the pen before undo/redo and ignores undo/redo during a drag.

### B10. Layers with shadows are re-recorded on every frame (found while checking)
- Cause: `renderer.ts` records a new SkPicture for each shadowed layer on every redraw.
- Fix: cache layer pictures like items, keyed by the hashes of the items inside.

## Settled design

- Engine (Rust → WASM) owns the Loro doc, layout and undo. React sends commands and
  reads snapshots.
- Engine emits one binary display list into WASM memory. A TS renderer reads it
  zero-copy and draws with CanvasKit (Skia, WebGL2), caching one SkPicture per
  item. The PDF writer (`krilla`) consumes the same display list.
- Text: `harfrust` shaping, `hypher` hyphenation, own Knuth-Plass. CanvasKit draws
  glyph IDs from the same font bytes.
- Color: document is RGB or CMYK + spot colors. Screen preview via `moxcms` with
  bundled PSO Coated v3 (FOGRA51).
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

### 8. Color
- Document color mode RGB or CMYK; color styles = swatches incl. spot colors.
- `moxcms` preview with PSO Coated v3.
- PDF: DeviceCMYK in CMYK documents, Separation for spot colors.

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
