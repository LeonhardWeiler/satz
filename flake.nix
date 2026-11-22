{
  description = "Satz development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust = pkgs.rust-bin.stable.latest.minimal.override {
          extensions = [ "clippy" "rustfmt" "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
        rustc-wrapper = pkgs.writeShellScript "rustc-wrapper" ''
          if [ -d "''${OUT_DIR-}" ]; then
            key=$(cd "$OUT_DIR" && find . -type f -print0 | sort -z | xargs -0r sha256sum | sha256sum | cut -c1-16)
            out=''${XDG_CACHE_HOME:-$HOME/.cache}/satz/out/$key
            if [ ! -d "$out" ]; then
              mkdir -p "''${out%/*}"
              cp -rT "$OUT_DIR" "$out.$$"
              mv -T "$out.$$" "$out" 2>/dev/null || :
            fi
            export OUT_DIR=$out
          fi
          exec ${pkgs.sccache}/bin/sccache "$@"
        '';
        e2e = {
          packages = with pkgs; [
            nodejs_24
            pnpm
            mupdf
          ];
          PLAYWRIGHT_BROWSERS_PATH = pkgs.playwright-driver.browsers.override {
            withChromium = false;
            withFirefox = false;
            withWebkit = false;
          };
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
          OXLINT_TSGOLINT_PATH = "${pkgs.tsgolint}/bin/tsgolint";
        };
      in {
        packages.canvaskit = pkgs.callPackage ./canvaskit.nix { };
        devShells.default = pkgs.mkShell (e2e // {
          packages = e2e.packages ++ (with pkgs; [
            rust
            wasm-pack
            wasm-bindgen-cli
            binaryen
            python3
            imagemagick
            perl
            librsvg
            typescript
            sccache
          ]);
          RUSTC_WRAPPER = rustc-wrapper;
        });
        devShells.e2e = pkgs.mkShell e2e;
      });
}
