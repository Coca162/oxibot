{
  description = "Build a cargo project without extra checks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      craneLib = crane.mkLib pkgs;

      # Common arguments can be set here to avoid repeating them later
      # Note: changes here will rebuild all dependency crates
      commonArgs = let
        # Only keeps markdown files
        sqlxFilter = path: _type: pkgs.lib.hasInfix "/.sqlx/" path || pkgs.lib.hasInfix "/migrations/" path;
        sqlxOrCargo = path: type:
          (sqlxFilter path type) || (craneLib.filterCargoSources path type);
      in {
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = sqlxOrCargo;
          name = "source"; # Be reproducible, regardless of the directory name
        };
        strictDeps = true;

        buildInputs =
          [
            # Add additional build inputs here
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
      };

      oxibot = craneLib.buildPackage (commonArgs
        // {
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          # Additional environment variables or build phases/hooks can be set
          # here *without* rebuilding all dependency crates
          # MY_CUSTOM_VAR = "some value";
        });
    in {
      checks = {
        oxibot = oxibot;
      };

      packages.default = oxibot;

      apps.default = flake-utils.lib.mkApp {
        drv = oxibot;
      };

      # devShells.default = craneLib.devShell {
      #   # Inherit inputs from checks.
      #   checks = self.checks.${system};

      #   # Additional dev-shell environment variables can be set directly
      #   # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

      #   # Extra inputs can be added here; cargo and rustc are provided by default.
      #   packages = [
      #     # pkgs.ripgrep
      #   ];
      # };
    });
}
