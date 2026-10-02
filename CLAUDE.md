# Satz

Rust engine (`engine/`, compiled to WASM) owns the Loro document; the React UI
(`web/`) sends `Command`s and reads `Snapshot`s. See `AGENT/TODO.md` for the design.

## Checks

`./check` runs everything CI runs, in this order: `cargo fmt --check`, clippy with
`-D warnings`, `cargo test`, the WASM build into `web/src/engine`, then in `web/`
tsc, eslint, vitest, vite build and Playwright. `./check build` stops before
Playwright, `./check e2e [args]` runs only Playwright against `web/dist`.
`web/src/testdata` holds the display-list fixture that vitest reads; `cargo test` fails when it
is stale, and `SATZ_WRITE_FIXTURES=1 cargo test fixture` writes it again.
`cargo test --release -p engine bench -- --ignored --nocapture` measures the engine
(`engine/src/doc/bench.rs`); it is not in CI.

Setup: `nix develop` provides all tools. Without nix: stable Rust with the
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
- Deleted nodes move under a `trash` root so that undo restores them with their ids.
