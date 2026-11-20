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
          ]);
        });
        devShells.e2e = pkgs.mkShell e2e;
      });
}
