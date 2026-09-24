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
        libPath = with pkgs; lib.makeLibraryPath [
          libGL
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXi
          xorg.libXcursor
          xorg.libXrandr
        ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "aquarium-espresso-rs";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];

          buildInputs = with pkgs; [
            libGL
            libxkbcommon
            wayland
            xorg.libX11
            xorg.libXi
            xorg.libXcursor
            xorg.libXrandr
          ];

          postInstall = ''
            wrapProgram $out/bin/aquarium-espresso-rs \
              --prefix LD_LIBRARY_PATH : "${libPath}"
          '';
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/aquarium-espresso-rs";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          packages = with pkgs; [
            rustc
            cargo
            rust-analyzer
          ];
          LD_LIBRARY_PATH = libPath;
        };
      }
    );
}
