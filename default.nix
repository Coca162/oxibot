{pkgs ? import <nixpkgs> {}}: let
  pins = import ./npins;
  inherit (pkgs) lib;
  craneLib = import pins.crane {inherit pkgs;};

  unfilteredRoot = ./.;
  src = lib.fileset.toSource {
    root = unfilteredRoot;
    fileset = lib.fileset.unions [
      ./.sqlx
      ./migrations
      (craneLib.fileset.commonCargoSources unfilteredRoot)
    ];
  };

  commonArgs = {
    inherit src;
    strictDeps = true;

    buildInputs = [pkgs.openssl];
  };
in
  craneLib.buildPackage commonArgs
  // {
    # Allow for reuse of previous dependency builds
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
  }
