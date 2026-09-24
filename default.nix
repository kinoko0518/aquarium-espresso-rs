{ lib
, rustPlatform
, pkg-config
, makeWrapper
, libGL
, libxkbcommon
, wayland
, libx11
, libxi
, libxcursor
, libxrandr
}:
let
  libPath = lib.makeLibraryPath [
    libGL
    libxkbcommon
    wayland
    libx11
    libxi
    libxcursor
    libxrandr
  ];
in
rustPlatform.buildRustPackage {
  pname = "aquarium-espresso-rs";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    makeWrapper
  ];

  buildInputs = [
    libGL
    libxkbcommon
    wayland
    libx11
    libxi
    libxcursor
    libxrandr
  ];

  postInstall = ''
    wrapProgram $out/bin/aquarium-espresso-rs \
      --prefix LD_LIBRARY_PATH : "${libPath}"
  '';
}
