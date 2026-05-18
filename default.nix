{pkgs ? import <nixpkgs> {}}: let
  inherit (pkgs) lib;
  crane = pkgs.fetchFromGitHub {
    owner = "ipetkov";
    repo = "crane";
    rev = "v0.23.4";
    hash = "sha256-nnGD2f8OlAZT2i5OfwikJsw+ifWfiA4d6A8BWlgOXV0=";
  };
  craneLib = import crane {inherit pkgs;};

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

    nativeBuildInputs = [pkgs.pkg-config];

    buildInputs = [pkgs.openssl];
  };
in
  craneLib.buildPackage commonArgs
  // {
    # Allow for reuse of previous dependency builds
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;
    meta.mainProgram = "oxibot";
  }
