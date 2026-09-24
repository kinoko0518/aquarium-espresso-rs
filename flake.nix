{
  description = "Aquarium Espresso ported to Rust + Macroquad";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        pkg = pkgs.callPackage ./default.nix {};
      in {
        packages.default = pkg;

        apps.default = {
          type = "app";
          program = "${pkg}/bin/aquarium-espresso-rs";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ pkg ];
          packages = with pkgs; [
            rustc
            cargo
            rust-analyzer
          ];
        };
      }
    );
}
