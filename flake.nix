{
  description = "Dev shell for libgl";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in
      with pkgs;
      rec {
        devShells.default = mkShell rec {

          nativeBuildInputs = [
            pkg-config
            cmake
          ];

          buildInputs = [
            libtool
            fontconfig

            vulkan-loader.out
            vulkan-headers

            fontconfig
            libxkbcommon
            libGL
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11

            (rust-bin.stable."1.83.0".default.override {
              extensions = [ "rust-src" ];
              targets = [ "wasm32-unknown-unknown" ];
            })

          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      }
    );
}
