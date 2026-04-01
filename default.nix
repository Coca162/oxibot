{pkgs ? import <nixpkgs> {}}: let
  inherit (pkgs) lib;
  crane = pkgs.fetchFromGitHub {
    owner = "ipetkov";
    repo = "crane";
    rev = "v0.23.2";
    hash = "sha256-BEp040fMklOW2+ttpNUKivlE0hzpv0AAP+0kG47uVCk=";
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
