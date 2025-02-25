{pkgs ? import <nixpkgs> {}}: let
  inherit (pkgs) lib;
  crane = pkgs.fetchzip {
    url = "https://github.com/ipetkov/crane/archive/19de14aaeb869287647d9461cbd389187d8ecdb7.tar.gz";
    sha256 = "0a8rbd0nc7iad7a5c1ask2kc7zhcigvhrrsf8zjidfal6d9352y7";
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
