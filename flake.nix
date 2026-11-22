{
  description = "arto-markdown - Markdown to HTML, and the frontend that draws it, for any host";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.xz";
    rust-overlay = {
      url = "git+https://github.com/oxalica/rust-overlay?shallow=1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # No packages: what this repository releases is a crate and an npm package,
  # which their registries build from source. The flake is the development
  # shell every `just` recipe runs in, locally and in CI.
  outputs =
    {
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          # The toolchain rust-toolchain.toml pins, wasm32 target included,
          # which rustup also reads where Nix does not run.
          rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          default = pkgs.mkShell {
            packages = [
              rust
              pkgs.just

              # The frontend under packages/ (`just assets`, `just test`).
              pkgs.nodejs_24
              pkgs.pnpm_10

              pkgs.nixfmt # `just fmt`, `just fmt-check`
              pkgs.cargo-hack # `just features`
              pkgs.cargo-deny # `just deny`
              pkgs.cargo-machete # `just machete`
              pkgs.cargo-insta # review rendering snapshots (`cargo insta review`)
              pkgs.lychee # `just docs`
              pkgs.actionlint # `just workflows`
              pkgs.zizmor # `just workflows`
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [ pkgs.libiconv ];
          };
        }
      );

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt);
    };
}
