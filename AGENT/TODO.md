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

Done; numbers and the items measured and not done in `AGENT/performance.md`.

## Later

- [ ] Figma import from an account; .fig files import already (`engine/src/fig.rs`, instance overrides, masks and clips ignored, text as outlines).
- [ ] InDesign import (IDML) beside SVG, PDF and Figma in `import::import`.
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
- [ ] Version history from the Loro history: browse and restore earlier states.
- [ ] GREP styles.
- [ ] Colour wheel in the picker for harmonies (analogous, complementary, …).
- [ ] AI connection for the common providers, as in Figma.
- [ ] With a server: work together on one project through a link or code.
