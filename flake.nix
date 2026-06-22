{
  description = "Solito development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rust-analyzer
            pkg-config
            clang
            nushell
          ];

          buildInputs = with pkgs; [
            wayland
            libxkbcommon
            libx11
            libxcursor
            libxi
            libxrandr
            libxext
            libxrender
            libxcb
            vulkan-loader
            mesa
            libGL
            fontconfig
            freetype
            expat
          ];

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [
              pkgs.wayland
              pkgs.libxkbcommon
              pkgs.libx11
              pkgs.libxcursor
              pkgs.libxi
              pkgs.libxrandr
              pkgs.libxext
              pkgs.libxrender
              pkgs.libxcb
              pkgs.vulkan-loader
              pkgs.mesa
              pkgs.libGL
              pkgs.fontconfig
              pkgs.freetype
              pkgs.expat
            ]}:$LD_LIBRARY_PATH
          '';
        };
      });
}
