{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix.url = "github:nix-community/fenix";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      flake-utils,
      crane,
      nixpkgs,
      fenix,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };

        craneLib = crane.mkLib pkgs;
      in
      {
        defaultPackage =
          let
            unfilteredRoot = ./.;
            src = pkgs.lib.fileset.toSource {
              root = unfilteredRoot;
              fileset = pkgs.lib.fileset.unions [
                (craneLib.fileset.commonCargoSources unfilteredRoot)
                (pkgs.lib.fileset.fileFilter (file: file.hasExt "css") unfilteredRoot)
              ];
            };

          in
          craneLib.buildPackage {
            # src = craneLib.cleanCargoSource ./.;
            inherit src;

            buildInputs = with pkgs; [
              gtk4
              pkg-config
              gtk4-layer-shell
            ];
          };

        devShell = pkgs.mkShell {
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            pkgs.gtk4
            pkgs.glib
            pkgs.gtk4-layer-shell
          ];
          nativeBuildInputs = with pkgs; [
            gtk4
            gtk4-layer-shell
            pkg-config
            rust-analyzer
            cargo-watch
            bacon
            (pkgs.fenix.stable.withComponents [
              "cargo"
              "clippy"
              "rust-src"
              "rustc"
              "rustfmt"
            ])
          ];
        };
      }
    );
}
