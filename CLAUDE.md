# Satz

Rust engine (`engine/`, WASM) owns the Loro document; the React UI (`web/`) sends
`Command`s and reads `Snapshot`s.

## Checks

`nix develop -c ./check` runs what CI runs. `./check build` stops before Playwright,
`./check e2e [args]` runs only Playwright against `web/dist`.
Before a commit: `./check build` and the affected specs; the full `./check` before the push.
`SATZ_WRITE_FIXTURES=1 cargo test fixture` writes the generated files in `web/src`
(`types.ts`, `testdata`, `unicode-names.txt`); `cargo test` fails when one is stale.

## Rules

- No comments.
- A keyboard shortcut is an `Action` in `web/src/commands.ts`; `onKey` in `web/src/keys.ts` is the only
  keydown dispatcher.
- Values both sides need live in Rust and reach TS through `types.ts`.
- An arm of `Doc::apply` checks everything before it writes to Loro.
- `wasm-bindgen` in `engine/Cargo.toml` matches the flake's `wasm-bindgen-cli`,
  `@playwright/test` the nixpkgs `playwright-driver`, skrifa krilla's.
- UI changes follow `AGENT/design.md`.
- Each agent works in its own git worktree and stages files by name, never `git commit -a`.
- Wait for CI with `watch_pull_request`, not by polling.
