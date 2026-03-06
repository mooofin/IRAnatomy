{
  description = "Nix development environment for llvm-ir-explorer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
        llvmPkgs = pkgs.llvmPackages;
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs;
            [
              rustc
              cargo
              rustfmt
              clippy
              rust-analyzer
              clang
              llvm
              graphviz
              binaryen
              wasm-bindgen-cli
              pkg-config
              openssl
            ]
            ++ lib.optionals (pkgs ? cargo-leptos) [ pkgs.cargo-leptos ];

          shellHook = ''
            export CC=clang
            export CXX=clang++
            export LIBCLANG_PATH="${llvmPkgs.libclang.lib}/lib"

            if ! command -v cargo-leptos >/dev/null 2>&1; then
              echo "cargo-leptos not found in nixpkgs; install it once with: cargo install cargo-leptos"
            fi
          '';
        };
      });
}
