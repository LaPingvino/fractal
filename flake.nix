# Fractal with the spaces work (F7, the Nix format): nixpkgs' own fractal
# package with the source swapped for LaPingvino/fractal's spaces branch.
#   nix run github:LaPingvino/fractal/build
# nixos-unstable, for GTK >= 4.20.2 and libadwaita >= 1.8.
{
  description = "Fractal with spaces support (AI-assisted fork branch)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fractal-src = {
      url = "github:LaPingvino/fractal/spaces";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, fractal-src }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: rec {
        fractal-spaces = pkgs.fractal.overrideAttrs (old: {
          pname = "fractal-spaces";
          version = "14.3-spaces-${fractal-src.shortRev or "dirty"}";
          src = fractal-src;
          # nixpkgs' patches are for its own release
          patches = [ ];
          # the crates, from the spaces branch's Cargo.lock (the CI prints the
          # new hash when the lock changes)
          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            src = fractal-src;
            name = "fractal-spaces-vendor";
            hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
          };
          # as in the other packages: line tables, or the build runs out of memory
          env = (old.env or { }) // { CARGO_PROFILE_RELEASE_DEBUG = "line-tables-only"; };
          meta = old.meta // {
            description = "Matrix group messaging app, with spaces support (AI-assisted fork branch)";
            homepage = "https://github.com/LaPingvino/fractal";
          };
        });
        default = fractal-spaces;
      });
    };
}
