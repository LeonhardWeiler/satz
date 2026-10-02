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

- The canvas composites transparency in RGB, the PDF of a CMYK document in CMYK:
  semi-transparent colour over dark CMYK colour looks lighter on the canvas.
- CMYK JPEGs: decoded to RGB and separated again; pass them through to the PDF
  as they are, and do not report them as RGB.
- Linked images: per document, chosen at creation, embed or link. Linking uses
  File System Access (Chromium) with handles in IndexedDB, so links need a click
  per session and are missing on other computers; Firefox shows the option
  disabled with a hint. Preflight reports missing links with relink.
- Local fonts (Local Font Access) in the font list. Today fonts are added by upload,
  and Local Font Access only finds missing ones.
- Type as in Figma: bulleted and numbered lists, truncation after a number of lines,
  vertical trim to cap height and baseline, axes of variable fonts, a glyph panel
  with search for all characters of a font.
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
- Threading: click an in-port to thread a frame in before another, as InDesign does. (versteh ich nicht ganz bitte erklären bevor es umgesetzt wird)
- Rotation: a turned multi-selection resized
  in its own frame, ports on turned text frames.
- Text wrap around layers, with an offset, for text frames under images and shapes.
- Find and replace across all stories, with text styles as a filter.
- Place SVG and PDF as vectors, kept as vectors in the exported PDF.
- Export pages as PNG or JPEG next to PDF.
- Package: one ZIP with the document, its fonts and images for the printer.
- Version history from the Loro history: browse and restore earlier states.
- Works offline as an installable PWA.
- Keep options in the flow: no widows and orphans, keep lines together, keep with
  next paragraph.
- Tabs with tab stops (left, right, centre, decimal) and a leader.
- Image fit: fill, fit and centre the image in its frame from the crop buttons.
- Overprint for fills and strokes, shown in the separations preview and written to
  the PDF.
- Sections: page numbering that starts at any number, in roman numerals or with a
  prefix.
- PDF options: page range, downsampling and JPEG compression of images.
- Spell check from Hunspell word lists in the engine, marked in the text.
- Superscript, subscript and baseline shift.
- Drop caps over a number of lines.
- More hyphenation languages (fr, it, es, nl).
