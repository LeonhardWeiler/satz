# Milestone 1: poster and booklet, engine → canvas → PDF with bleed

Done when: on the live GitHub Pages build, a user creates an A2 poster and an
8-page A5 booklet that use every M1 feature (master pages, threaded text, effects,
CMYK + spot color), and exports both as PDF with TrimBox, BleedBox and crop marks
that match the canvas.

Out of scope for M1: PDF/X, components, realtime collaboration, Figma import,
font helper, i18n.

## UI design G (branch ui-g, spec AGENT/ui-ux/g-register.html)

Done: G look, fixed layout, start screen, "." page overview, navigation keys,
fields (expressions, units, step, scrub), collapsible properties with document
and page sections, text style specimen menu. Open, in this order:

- Quick edit above the selection (`web/src/Quick.tsx`): built, has no E2E test yet
  (shown on selection, hidden while dragging, editing text and in modes, fill
  swatch menu, More opens and focuses the properties).
- Layers: canvas hover highlight, visible/lock buttons on hover, hidden rows
  dimmed, keys ↑↓→← Enter Shift+Enter Tab Shift+Tab F2 Ctrl+R, master row, empty
  state; drag & drop stays.
- Engine props `hidden` and `locked` (set, undo, old files load false): hidden is
  not drawn, exported, hit or preflighted; locked is not hit but selectable in the
  tree; parents and masters pass them on. Ctrl+Shift+H, Ctrl+Shift+L. Rust and E2E
  tests.
- Ctrl+K palette (commands, layers, pages, swatches, text styles; fuzzy; keys
  shown), "?" help, toasts incl. apply errors.
- Swatches as in G: chip, name, CMYK, RGB-only warning, click fills.
- Snapping (`snap.ts` with vitest): objects, page, centre, margins, columns,
  baseline grid within 5 px; move, resize, draw; Ctrl off, Shift locks the axis;
  distances to the 2 nearest objects, Alt under the pointer; magenta, mm; rulers
  show extent and guides.
- Preflight mode (Ctrl+Alt+Y, count button, Ctrl+Shift+E, Export; Esc; 384 px):
  issues incl. gamut, ink limit, <3 mm from trim; proof with gamut switch;
  separations C/M/Y/K/spots with max; ink coverage with limit, marking, max,
  value under the pointer; export presets X-4, X-1a, screen; crop/bleed via
  SetDocument; export button with error count, progress and toast.
- PDF presets in the engine (krilla), Rust structure tests; canvas-vs-PDF stays
  green with X-4.
- Every core.js shortcut, main's stay (stop on a collision).
- Docs: settled design here, README screenshot, CLAUDE.md contracts, screenshots
  at 1440×900 and 880×700 in AGENT/ui-ux/g-impl/; tick AGENT/ui-g-inventory.md.
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
- UI: design G (`AGENT/ui-ux/g-register.html`): fixed layout with a top bar, layers
  and swatches left, properties or preflight right, mm rulers; no floating panels, no
  bottom bar. Figma keybinds (Ctrl for Cmd), dark look, English, units mm by default
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
