# Milestone 1: one poster, engine → canvas → PDF with bleed

Done when: a user on Linux Chrome/Firefox opens the GitHub Pages build, sets up an
A2 poster with 3 mm bleed, places a text frame, an image and a rectangle in RGB or
CMYK colors, sees it on the canvas, reloads without losing it, and exports a PDF
with TrimBox, BleedBox and crop marks whose output matches the canvas.

Out of scope for M1: multi-page, master pages, frame threading, PDF/X, ICC preview,
Local Font Access, collaboration.

## 1. Workspace scaffold

- Cargo workspace with one crate `engine` (`cdylib` + `rlib`), built with
  `wasm-pack build --target web` into `web/src/engine`.
- `web/`: Vite + React + TypeScript, pnpm.
- One `check` entry point (`cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`, `tsc --noEmit`, `pnpm lint`) that CI and commits both use.
- GitHub Actions: run checks, deploy `web/dist` to GitHub Pages.

## 2. Document model (Loro)

- Loro doc schema: document (color mode RGB/CMYK, swatches incl. spot colors),
  one page (trim size, bleed), items (text frame, image frame, rectangle) with
  geometry in pt, z-order, fill/stroke swatch refs.
- Engine owns the Loro doc; React reads a snapshot and sends commands.
- Undo/redo via Loro `UndoManager`.
- Tests: create, edit, undo, export/import roundtrip.

## 3. Display list

- One engine-internal display list (paths, glyph runs, images, colors as
  document color values) built from the document.
- Canvas renderer and PDF writer both consume it; nothing else draws.

## 4. Text (single frame)

- Font upload plus one bundled open font (e.g. Source Serif 4).
- Shaping with `harfrust`, hyphenation with `hypher`.
- Own Knuth-Plass line breaking into one rectangular frame; overset detection.
- Paragraph attributes: size, leading, alignment (left, justified), language.
- TDD against fixed fonts and texts.

## 5. Canvas rendering

- `vello_hybrid` with the `webgl` feature into a `<canvas>`; `vello_cpu` fallback
  if WebGL2 is unavailable. WebGPU path after M1.
- Pan, zoom, HiDPI; show trim, bleed and margin guides.
- CMYK and spot colors shown via naive conversion to sRGB (ICC preview after M1).

## 6. Editing

- Hit testing and selection in the engine; handles for move and resize.
- Tools: select, text frame, rectangle, image frame.
- In-frame text editing: caret, selection, typing, IME via a hidden input.

## 7. Editor shell (React)

- Toolbar, layers panel (order, visibility, lock), properties panel
  (geometry, colors, paragraph attributes), document setup dialog (size, bleed,
  color mode), swatches panel.

## 8. Images

- Place PNG/JPEG; keep original bytes for PDF.
- Fit/fill within the frame, crop by moving content.
- Effective resolution (ppi) per placed image.

## 9. Persistence

- Autosave the Loro snapshot to IndexedDB.
- Export/import a `.satz` project file (Loro snapshot + fonts + images).

## 10. PDF export (`krilla`)

- Page = trim + bleed; set TrimBox and BleedBox; crop marks outside the bleed.
- Colors: DeviceCMYK in CMYK documents, Separation for spot colors, sRGB otherwise.
- Embedded font subsets, original image bytes.
- Tests: page boxes, color operators, fonts present; check with `qpdf --check`
  and a visual diff against the canvas render.

## 11. Preflight (minimal)

- Overset text, images below 300 ppi, objects touching the trim edge that stop
  short of the bleed, RGB content in a CMYK document.
- Panel listing issues; click selects the item. Export warns but does not block.

## 12. Ship

- Example poster in the repo, built by a test and exported to PDF.
- README with screenshot and live link.
