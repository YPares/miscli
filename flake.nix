{
  description = "Rust CLI/TUI utilities, by YPares";

  inputs.nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst";

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      lib = nixpkgs.lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          mkPackage =
            crateName:
            let
              tomlPackage = (lib.importTOML ./${crateName}/Cargo.toml).package;
            in
            pkgs.rustPlatform.buildRustPackage {
              inherit (tomlPackage) version;
              pname = tomlPackage.name;
              src = ./.;
              cargoLock.lockFile = ./Cargo.lock;
              cargoBuildFlags = [
                "--package"
                crateName
              ];
              cargoTestFlags = [
                "--package"
                crateName
              ];
              meta = {
                inherit (tomlPackage) description;
                mainProgram = crateName;
              };
            };
        in
        {
          rscat = mkPackage "rscat";
        }
      );

      apps = forAllSystems (
        system:
        let
          selfPkgs = self.packages.${system};
          mkApp = crateName: {
            type = "app";
            program = "${selfPkgs.${crateName}}/bin/${crateName}";
          };
        in
        {
          rscat = mkApp "rscat";
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              rustfmt
              clippy
              rust-analyzer
            ];
          };
        }
      );
    };
}
