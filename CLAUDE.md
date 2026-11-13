# Satz

Rust engine (`engine/`, compiled to WASM) owns the Loro document; the React UI
(`web/`) sends `Command`s and reads `Snapshot`s. See `AGENT/TODO.md` for the design.

## Checks

`./check` runs everything CI runs, in this order: `cargo fmt --check`, clippy with
`-D warnings`, `cargo test`, the WASM build into `web/src/engine`, then in `web/`
tsc, eslint, vitest, vite build and Playwright. `./check build` stops before
Playwright, `./check e2e [args]` runs only Playwright against `web/dist`, served on
`PORT` (default 4173).
Before a commit run `./check build` and the affected specs (`./check e2e e2e/<name>.spec.ts`);
the full `./check` runs once before the push.
`web/src/testdata` holds the display-list fixture that vitest reads, `web/src/types.ts` the engine's types (ts-rs) and `web/src/unicode-names.txt`
the names the characters panel fetches; `cargo test` fails when one
is stale, and `SATZ_WRITE_FIXTURES=1 cargo test fixture` writes them again.
`cargo test --release -p engine bench -- --ignored --nocapture` measures the engine
(`engine/src/doc/bench.rs`); it is not in CI.

Setup: `nix develop` provides all tools, `nix develop .#e2e` only those of `./check e2e`. Without nix: stable Rust with the
`wasm32-unknown-unknown` target, `wasm-pack`, `wasm-bindgen-cli` 0.2.127, binaryen,
Node 22, pnpm, MuPDF (`mutool`, for the PDF tests) and
`pnpm -C web exec playwright install chromium`.

## Rules

- Every arm of `Doc::apply` checks everything before it writes to Loro; a command
  that fails must leave no pending ops (a debug assertion and
  `a_command_that_fails_leaves_the_document_as_it_was` enforce it). Numbers in
  commands must be finite.
- The display list is a binary contract: `engine/src/display_list.rs` and
  `web/src/displayList.ts` change together.
- `wasm-bindgen` is pinned with `=` in `engine/Cargo.toml` to the version of the
  flake's `wasm-bindgen-cli`; bump both at once.
- `@playwright/test` is pinned to the version of `playwright-driver` in the flake's
  nixpkgs; bump it together with `nix flake update nixpkgs`. skrifa stays on the
  version krilla uses.
- A thread's story lives in its first frame; frames link on with `next`.
- Pages draw their master under their own layers. A page lists the master layers it
  overrides in `detached`, and each overriding copy names its master layer in
  `overrideOf`; a page that leaves its master drops both.
- Switching the document's colour mode converts every RGB or CMYK colour (nodes, text,
  swatches, variables, text styles) into it; spot colours, swatch and variable
  references stay as they are.
- `engine/icc/FOGRA51.icc` is built by `engine/icc/build` from the ICC registry data;
  PSO Coated v3 may not be redistributed. A document may hold an uploaded CMYK profile
  instead (`profile` in the `document` map); `Doc::finish` makes it the thread's
  profile in `color.rs`.
- A node's `hidden` and `locked` (default false) pass to its children and hold on
  masters too: hidden is not drawn, exported, hit or preflighted; locked is drawn but
  only the layers tree selects it.
- The preflight's separations, ink coverage and gamut marks come from `Inks`
  (`engine/src/inks.rs`), rasterized in the export worker, never on the main thread.
- PDF presets: screen (RGB, trim box only), PDF/X-4 and PDF/X-1a (CMYK and spots
  only, transparency flattened at the raster ppi, CMYK documents only). krilla writes
  the PDF; `engine/src/pdfx.rs` adds the profile's OutputIntent, XMP and Info in an
  incremental update. RGB X-4 pages blend in an sRGB group, CMYK documents blend in
  CMYK, on the canvas too (plates and a LUT of the profile in `renderer.ts`).
- A snapshot keeps the pages and masters of the last one that no change reaches; a
  subscription to the Loro document collects the changed nodes, any other change
  builds every page again.
- Deleted nodes move under a `trash` root so that undo restores them with their ids.
- A text frame wraps around the shown layers above it on its page whose `wrap` is
  set (`around` or `jump`, `wrapOffset` pt away); auto width text does not wrap.
- An anchored layer follows the object replacement character of a story whose `object`
  mark names it, inline (the line makes room for it) or `custom` (it keeps its offset
  to `anchorAt`); `Doc::finish` places it, and it wraps no text.
- A table is a frame with a `table` (column widths, least row heights); its cells are text
  layers with a `cell` ([column, row, columns, rows]). `lay_out_table` places them, rows
  growing to the text of unthreaded cells; `reframe` scales the table.
- An image of the `images` map is its file or, linked, a map `{link: name}`; this browser
  keeps its file's handle by hash. `Doc::register_image` reads a link's file for the
  session (the export worker gets the read files too), `relink` puts another read image in
  its place and `embed` keeps its file in the document.
- A span's `axes` set its variable font at a location; `text::font_of` gives the run a
  `VARIED` font id, which the canvas draws as outlines from `Engine::outline` and krilla
  instances in the PDF.
- A span's `condition` names a boolean variable; where it is false in the text's mode the
  span is `hidden` and `text::set` lays out the rest (`Shown`), mapping bytes back.
- Saved documents keep their Loro history. A version of `Doc::versions` is a Loro change
  (a peer's commits within 60 s merge); `Restore` is a `revert_to` whose commits, layout
  too, carry the message `restore`, so that undo takes it back as one.
